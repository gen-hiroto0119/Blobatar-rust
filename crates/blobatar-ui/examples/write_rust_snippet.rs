use blobatar_core::{Background, Expression};
use blobatar_ui::{editor::EditorState, snippet::snippet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Provide an output .rs path")?;
    let mut state = EditorState::default();
    state.settings.generation = 1;
    state.settings.name = "ひろと \"# 🦀\n".into();
    state.settings.options.expression = Some(Expression::Love);
    state.settings.options.background = Some(Background::Kind("squircle".into()));
    state.apply_traits_json(r#"{"shape":[0.11,0.965],"eye.gap":0.751,"hue":0.123}"#)?;
    let code = snippet(&state);
    let source = format!(
        "use blobatar_gpui::gpui::{{AppContext, Context, Entity}};\n\npub fn build(cx: &mut Context<()>) -> Result<Entity<blobatar_gpui::AnimatedBlobatar>, Box<dyn std::error::Error>> {{\n{code}\nOk(avatar)\n}}\nfn main() {{}}\n"
    );
    std::fs::write(path, source)?;
    Ok(())
}
