//! #8266 through the real Session edit/undo/redo path, without rendering or a large image.
use photocraft_doc::{BlendMode, ColorMode, Document, Layer, LayerContent, SampleType, Size, SmartFilter, SmartObject, SmartSource};
use photocraft_engine::Session;
use serde_json::json;
use std::sync::Arc;

fn payload(doc: &Document) -> &str {
    let LayerContent::Smart(smart) = &doc.layers[0].content else { panic!("fixture") };
    smart.smart_filters[0].params["__displacePsd"].as_str().unwrap()
}

#[test]
fn session_edits_share_and_budget_embedded_filter_payloads() {
    let mut doc = Document::new("original", Size::new(8, 8), ColorMode::Rgb, SampleType::U8);
    let mut smart = SmartObject::new(SmartSource::Embedded { file_name: "synthetic".into(), bytes: Arc::new(Vec::new()) }, Default::default(), None);
    smart.smart_filters.push(SmartFilter {
        command: "filter.distort.displace".into(),
        params: json!({"__displacePsd": "ab".repeat(8192)}).into(),
        blend: BlendMode::Normal,
        opacity: 1.0,
        visible: true,
    });
    doc.layers.push(Layer::new("smart", LayerContent::Smart(smart)));
    let layer = doc.layers[0].id;
    let mut session = Session::default();
    session.add_document(doc, None);
    let bytes = session.active().unwrap().memory_bytes();
    assert!(bytes >= 16384, "embedded map must count before the first edit");
    session.active_mut().unwrap().history.max_bytes = bytes;
    for i in 0..4 {
        session.execute("layer.renameLayer", json!({"layer": layer.0, "name": format!("rename {i}")})).unwrap();
    }
    let state = session.active().unwrap();
    assert_eq!(state.history.past_len(), 4);
    assert_eq!(state.memory_bytes(), bytes);
    for i in 0..4 {
        assert_eq!(payload(&state.history.state(i).unwrap()).as_ptr(), payload(&state.doc).as_ptr());
    }
    assert!(session.undo());
    assert_eq!(session.active().unwrap().doc.layer(layer).unwrap().name, "rename 2");
    assert_eq!(session.active().unwrap().memory_bytes(), bytes);
    assert!(session.redo());
    assert_eq!(session.active().unwrap().doc.layer(layer).unwrap().name, "rename 3");
    assert_eq!(session.active().unwrap().memory_bytes(), bytes);
}
