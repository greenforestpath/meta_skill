use ms::core::spec_lens::parse_markdown;
use ms::templates::{TemplateContext, find_template, render_template};

#[test]
fn template_renders_and_parses() {
    let template = find_template("debugging").expect("template exists");
    let ctx = TemplateContext {
        id: "debug-rust-builds".to_string(),
        name: "Debug Rust Builds".to_string(),
        description: "Diagnose Rust build failures and fix compiler errors.".to_string(),
        tags: vec!["rust".to_string(), "build".to_string()],
    };

    let rendered = render_template(template, &ctx).unwrap();
    let spec = parse_markdown(&rendered).unwrap();

    assert_eq!(spec.metadata.id, "debug-rust-builds");
    assert_eq!(spec.metadata.name, "Debug Rust Builds");
    assert!(spec.metadata.description.contains("Diagnose Rust build"));
    assert!(!spec.sections.is_empty());
}
