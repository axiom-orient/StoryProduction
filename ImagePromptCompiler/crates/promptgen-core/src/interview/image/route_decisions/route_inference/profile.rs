//! Inference of a typed render profile after category admission.

use std::collections::BTreeMap;

use crate::interview::answer;
use crate::interview::image::inference::infer_profile;

pub(in crate::interview::image) fn infer_profile_from_brief(
    brief: &str,
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    let Some(category) = answer(answers, "image.category") else {
        return;
    };
    if answer(answers, "image.profile").is_some() {
        return;
    }
    if let Some(profile) = infer_profile(brief, category) {
        answers.insert("image.profile".to_owned(), profile.to_owned());
        inferences.push(format!(
            "요청의 세부 의도를 근거로 image.profile={profile}을 추정했다."
        ));
    }
}
