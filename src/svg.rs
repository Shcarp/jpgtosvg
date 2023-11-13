use visioncortex::{Color, CompoundPath, PointF64};
use wasm_bindgen::JsValue;
use web_sys::{Blob, BlobPropertyBag, Element, Url, XmlSerializer, js_sys::JSON};

pub fn window() -> web_sys::Window {
    web_sys::window().unwrap()
}

pub fn document() -> web_sys::Document {
    window().document().unwrap()
}

pub struct Svg {
    pub paths: Vec<String>,
    pub options: SvgOptions,
    element: Element,
}
pub struct SvgOptions {
    pub scale: f32,
    pub backgroundColor: Option<String>,
    pub pathFill: Option<String>,
    pub attributes: Option<String>,
}
/**
Constructs a "dumb" string only svg.
Real elements aren't used so that this can run in a webworker.
*/
impl Svg {
    pub fn new(options: SvgOptions) -> Self {
        let paths = vec![];
        Self {
            paths,
            options,
            element: document().create_element("svg").unwrap(),
        }
    }

    pub fn get_svg_string(&self) -> String {

		let sequence = XmlSerializer::new().unwrap().serialize_to_string(&self.element).unwrap();

        let blob = Blob::new_with_str_sequence_and_options(
			&JsValue::from_str(&sequence),
			BlobPropertyBag::new().type_("image/svg+xml"),
		);

		match blob {
			Ok(blob) => {
				Url::create_object_url_with_blob(&blob).unwrap()
			},
			Err(e) => format!("{:#?}", e),
		}
    }

    pub fn prepend_path(&mut self, paths: &CompoundPath, color: &Color, precision: Option<u32>) {
        let path = document()
            .create_element_ns(Some("http://www.w3.org/2000/svg"), "path")
            .unwrap();
        let (string, offset) = paths.to_svg_string(true, PointF64::default(), precision);
        path.set_attribute("d", &string).unwrap();
        path.set_attribute(
            "transform",
            format!("translate({},{})", offset.x, offset.y).as_str(),
        )
        .unwrap();
        path.set_attribute(
            "style",
            format!("fill: {};", color.to_hex_string()).as_str(),
        )
        .unwrap();
        self.element.prepend_with_node_1(&path).unwrap();
    }
}
