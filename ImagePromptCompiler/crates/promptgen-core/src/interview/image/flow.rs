use std::collections::BTreeMap;

use super::super::{InterviewOutcome, InterviewRequest, InterviewRequestValue, normalized_answers};
use super::builder::build_request_json;
use super::route_decisions::category_contracts::answer_pruning::prune_irrelevant_category_answers;
use super::route_decisions::category_contracts::category::infer_category_contract_from_brief;
use super::route_decisions::category_contracts::profile::infer_medium_from_route;
use super::route_decisions::category_contracts::questions::{
    category_routing_question, collect_routing_questions,
};
use super::route_decisions::route_inference::content::infer_route_content;
use super::route_decisions::route_inference::profile::infer_profile_from_brief;
use super::route_decisions::route_inference::route::infer_route_from_brief;
use super::route_decisions::route_inference::stated_fields::infer_stated_content_from_brief;
use super::text_contract::{apply_text_defaults, collect_contract_questions, resolve_text_mode};
use super::visual_defaults::{apply_visual_defaults, collect_validation_questions};
use crate::diagnostic::{CompilationOutcome, Diagnostic, PromptKind};
use crate::image::{ImagePromptRequest, compile_image_prompt};
use crate::interview::InterviewQuestion;
use crate::json::JsonValue;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ImageInterviewStage {
    /// Only the route itself is decided here: output language and the category
    /// (plus a category-compatible supporting style). Nothing route-dependent may
    /// be written before the category is known to be valid, otherwise a rejected
    /// selection would leave inferred content behind for the next turn.
    Route,
    /// Route-dependent inference, reached only after the category is accepted.
    Infer,
    Routing,
    TextContract,
    VisualDefaults,
    Validation,
    Compile,
}

struct ImageInterviewState {
    stage: ImageInterviewStage,
    answers: BTreeMap<String, String>,
    inferences: Vec<String>,
}

enum ImageInterviewStep {
    Advance(ImageInterviewStage),
    NeedsInput(Vec<InterviewQuestion>),
    Compile,
}

impl ImageInterviewState {
    fn new(request: &InterviewRequest) -> Self {
        Self {
            stage: ImageInterviewStage::Route,
            answers: normalized_answers(request),
            inferences: Vec::new(),
        }
    }

    fn step(&mut self, request: &InterviewRequest) -> ImageInterviewStep {
        match self.stage {
            ImageInterviewStage::Route => {
                infer_route_from_brief(&request.brief, &mut self.answers, &mut self.inferences);
                if let Some(question) = category_routing_question(&self.answers) {
                    return ImageInterviewStep::NeedsInput(vec![question]);
                }
                ImageInterviewStep::Advance(ImageInterviewStage::Infer)
            }
            ImageInterviewStage::Infer => {
                // What the brief states comes first. Every inference step only fills an
                // absent answer, so running the route's default medium ahead of the
                // brief would let `photo` win over an explicitly requested illustration.
                infer_stated_content_from_brief(
                    &request.brief,
                    &mut self.answers,
                    &mut self.inferences,
                );
                prune_irrelevant_category_answers(&mut self.answers, &mut self.inferences);
                infer_category_contract_from_brief(
                    &request.brief,
                    &mut self.answers,
                    &mut self.inferences,
                );
                infer_profile_from_brief(&request.brief, &mut self.answers, &mut self.inferences);
                infer_medium_from_route(&mut self.answers, &mut self.inferences);
                infer_route_content(&request.brief, &mut self.answers, &mut self.inferences);
                ImageInterviewStep::Advance(ImageInterviewStage::Routing)
            }
            ImageInterviewStage::Routing => {
                let questions = collect_routing_questions(&self.answers);
                if questions.is_empty() {
                    ImageInterviewStep::Advance(ImageInterviewStage::TextContract)
                } else {
                    ImageInterviewStep::NeedsInput(questions)
                }
            }
            ImageInterviewStage::TextContract => {
                resolve_text_mode(&mut self.answers, &mut self.inferences);
                apply_text_defaults(&mut self.answers, &mut self.inferences);
                let questions = collect_contract_questions(&self.answers);
                if questions.is_empty() {
                    ImageInterviewStep::Advance(ImageInterviewStage::VisualDefaults)
                } else {
                    ImageInterviewStep::NeedsInput(questions)
                }
            }
            ImageInterviewStage::VisualDefaults => {
                apply_visual_defaults(&request.brief, &mut self.answers, &mut self.inferences);
                ImageInterviewStep::Advance(ImageInterviewStage::Validation)
            }
            ImageInterviewStage::Validation => {
                let questions = collect_validation_questions(&self.answers);
                if questions.is_empty() {
                    ImageInterviewStep::Advance(ImageInterviewStage::Compile)
                } else {
                    ImageInterviewStep::NeedsInput(questions)
                }
            }
            ImageInterviewStage::Compile => ImageInterviewStep::Compile,
        }
    }

    fn needs_input(self, questions: Vec<InterviewQuestion>) -> InterviewOutcome {
        InterviewOutcome::needs_input(PromptKind::Image, self.answers, self.inferences, questions)
    }

    fn compile(self, request: &InterviewRequest) -> InterviewOutcome {
        let request_json = build_request_json(request, &self.answers);
        match ImagePromptRequest::from_json(request_json.clone()) {
            Ok(decoded) => {
                let compilation = compile_image_prompt(&decoded);
                InterviewOutcome::completed(
                    PromptKind::Image,
                    self.answers,
                    self.inferences,
                    request_json,
                    Some(InterviewRequestValue::Image(Box::new(decoded))),
                    compilation,
                )
            }
            Err(error) => {
                let compilation = CompilationOutcome::new(
                    PromptKind::Image,
                    None,
                    vec![Diagnostic::error(
                        "INTERVIEW_REQUEST_BUILD",
                        "$",
                        error.to_string(),
                    )],
                    JsonValue::Object(Default::default()),
                );
                InterviewOutcome::completed(
                    PromptKind::Image,
                    self.answers,
                    self.inferences,
                    request_json,
                    None,
                    compilation,
                )
            }
        }
    }
}

pub(super) fn run(request: &InterviewRequest) -> InterviewOutcome {
    let mut state = ImageInterviewState::new(request);
    loop {
        match state.step(request) {
            ImageInterviewStep::Advance(next) => state.stage = next,
            ImageInterviewStep::NeedsInput(questions) => return state.needs_input(questions),
            ImageInterviewStep::Compile => return state.compile(request),
        }
    }
}
