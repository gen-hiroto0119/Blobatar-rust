use blobatar_gpui::gpui::{AppContext, Context, Entity};

pub fn build(
    cx: &mut Context<()>,
) -> Result<Entity<blobatar_gpui::AnimatedBlobatar>, Box<dyn std::error::Error>> {
    use blobatar_core::{Generation, Options};
    use blobatar_gpui::{Animate, AnimatedBlobatar};

    let options: Options = serde_json::from_str(
        r#"{
  "background": "squircle",
  "traits": {
    "eye.gap": 0.751,
    "hue": 0.123,
    "shape": [
      0.11,
      0.965
    ]
  },
  "expression": "love"
}"#,
    )?;
    let avatar = cx.new(|cx| {
        let mut avatar =
            AnimatedBlobatar::with_generation("ひろと \"# 🦀\n", &options, Generation::One)
                .size(192.0);
        avatar.set_animate(Animate::Hover, cx);
        avatar.set_reduced_motion(false, cx);
        avatar
    });
    Ok(avatar)
}
fn main() {}
