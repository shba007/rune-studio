pub use rune_infer::model_management::Downloads;

fn models(
    memory: Option<u64>,
    gpu: bool,
    disk: Option<u64>,
) -> Result<Vec<rune_infer::model_management::Candidate>, String> {
    let ids = [
        "qwen3.8-27b",
        "qwen3.5-4b-q4",
        "holo3.1-0.8b-q4",
        "holo3.1-4b-q4",
        "holo3.1-9b-q4",
        "holo4-27b-q4",
    ]
    .map(str::to_owned);
    let holo_ids: Vec<_> = ids
        .iter()
        .filter(|id| id.starts_with("holo"))
        .cloned()
        .collect();
    let mut models = rune_infer::model_management::candidates(&ids, memory, gpu, disk)
        .map_err(|error| format!("Cannot load model list: {error:#}"))?;
    let recommendations = rune_infer::model_management::candidates(&holo_ids, memory, gpu, disk)
        .map_err(|error| format!("Cannot load model recommendations: {error:#}"))?;
    for model in &mut models {
        model.recommended = recommendations
            .iter()
            .any(|candidate| candidate.id == model.id && candidate.recommended);
    }
    Ok(models)
}

pub fn candidates() -> Result<Vec<rune_infer::model_management::Candidate>, String> {
    let root = rune_infer::home::rune_home().map_err(|error| error.to_string())?;
    if !root.is_absolute() {
        return Err("RUNE_HOME must be an absolute path".into());
    }
    let (memory, gpu) = rune_infer::model_management::detected_memory();
    let disk = rune_infer::model_management::available_disk(&root);
    models(memory, gpu, disk)
}

#[cfg(test)]
mod tests {
    #[test]
    fn recommends_a_holo_model_even_when_qwen_also_fits() {
        let models = super::models(Some(32 * 1024 * 1024 * 1024), true, None).unwrap();
        let selected: Vec<_> = models.iter().filter(|model| model.recommended).collect();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "holo4-27b-q4");
    }
}
