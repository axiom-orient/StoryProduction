use crate::error::{Error, Result};
use crate::model::{Project, Shot, VisualPlan};

pub fn plan_scene(project: &Project, scene_id: &str, plan_id: String) -> Result<VisualPlan> {
    let scene = project
        .scenes
        .get(scene_id)
        .ok_or_else(|| Error::NotFound(format!("scene '{scene_id}' was not found")))?;
    let mut shots: Vec<_> = scene
        .text
        .split("\n\n")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .enumerate()
        .map(|(index, paragraph)| Shot {
            index,
            shot_type: if index == 0 {
                "establishing".into()
            } else if paragraph.contains('"') || paragraph.contains('“') || paragraph.contains('‘')
            {
                "dialogue".into()
            } else {
                "medium".into()
            },
            description: truncate(paragraph, 180),
        })
        .collect();
    if shots.is_empty() {
        shots.push(Shot {
            index: 0,
            shot_type: "establishing".into(),
            description: scene.title.clone(),
        });
    }
    Ok(VisualPlan {
        id: plan_id,
        scene_id: scene_id.into(),
        source_revision: project.content_revision,
        shots,
    })
}

fn truncate(value: &str, limit: usize) -> String {
    let mut characters = value.chars();
    let mut result: String = characters.by_ref().take(limit).collect();
    if characters.next().is_some() {
        result.push('…');
    }
    result
}
