#![allow(non_snake_case)]
#![allow(unused_parens)]
use web_sys::{console, ImageData};
mod svg;
mod utils;
use serde::{Deserialize, Serialize};
use svg::*;
use tsify::Tsify;
use visioncortex::{
	ColorImage, PathSimplifyMode, Color, color_clusters::KeyingAction,
};
use wasm_bindgen::prelude::*;
use visioncortex::color_clusters::{IncrementalBuilder, Clusters, Runner, RunnerConfig, HIERARCHICAL_MAX};

#[allow(dead_code)]
fn log(string: &str) { console::log_1(&wasm_bindgen::JsValue::from_str(string)); }

#[wasm_bindgen(start)]
pub fn main() {
	utils::set_panic_hook();
	console_log::init().unwrap();
}
pub fn path_simplify_mode(s: &str) -> PathSimplifyMode {
	match s {
		"polygon" => PathSimplifyMode::Polygon,
		"spline" => PathSimplifyMode::Spline,
		"none" => PathSimplifyMode::None,
		_ => panic!("unknown PathSimplifyMode {}", s),
	}
}

#[derive(Tsify, Debug, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct RawImageData {
	#[tsify(type = "Uint8ClampedArray")]
	pub data: Vec<u8>,
	pub width: usize,
	pub height: usize,
}

#[derive(Debug)]
pub struct DebugImageData {
	pub data_len: usize,
	pub first_val: bool,
	pub width: usize,
	pub height: usize,
}

// these are the defults used in vtracer's demo app

fn default_mode() -> String { "spline".to_string() }
fn default_scale() -> f32 { 1.0 }
fn default_cornerThreshold() -> f64 { 60.0_f64.to_radians() }
fn default_lengthThreshold() -> f64 { 4.0 }
fn default_maxIterations() -> usize { 10 }
fn default_spliceThreshold() -> f64 { 45.0_f64.to_radians() }
fn default_filterSpeckle() -> usize { 4 }
fn default_pathPrecision() -> u32 { 8 }

#[derive(Tsify, Debug, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ColorImageConverterParams {
	pub debug: Option<bool>,
	/** Default is spline. none = pixel. */
	#[tsify(type = "'polygon'|'spline'|'none'")]
	#[serde(default = "default_mode")]
	pub mode: String,
	/** Must be in radians. Default is 60deg */
	#[serde(default = "default_cornerThreshold")]
	pub cornerThreshold: f64,
	/** Default is 4. */
	#[serde(default = "default_lengthThreshold")]
	pub lengthThreshold: f64,
	/** Default is 10. */
	#[serde(default = "default_maxIterations")]
	pub maxIterations: usize,
	/** Must be in radians. Default is 45deg */
	#[serde(default = "default_spliceThreshold")]
	pub spliceThreshold: f64,
	/** Default is 4. */
	#[serde(default = "default_filterSpeckle")]
	pub filterSpeckle: usize,
	/** Default is 8. */
	#[serde(default = "default_pathPrecision")]
	pub pathPrecision: u32,
	pub layer_difference: i32,
	pub filter_speckle: usize,
	pub color_precision: i32,
	pub hierarchical: String,
	pub corner_threshold: f64,
    pub length_threshold: f64,
    pub max_iterations: usize,
    pub splice_threshold: f64,
	pub path_precision: u32,
}

#[derive(Tsify, Debug, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct Options {
	/** Process an inverted version of the image. */
	pub invert: Option<bool>,
	/** The color to set for the path fill property. By the default this is the color returned by visioncortex's binary converter (i.e. black).*/
	pub pathFill: Option<String>,
	/** The color given to the svg element background, white by default. This is set in a style tag.*/
	pub backgroundColor: Option<String>,
	/** Additional attributes to add to the svg. For now this is a string to simplify things, therefore you cannot specify a style tag, or if you do, you're overriding the default one which contains the background color.*/
	pub attributes: Option<String>,
	/** Create a group and scale the final svg by this amount.*/
	#[serde(default = "default_scale")]
	pub scale: f32,
}

pub enum Stage {
    New,
    Clustering(IncrementalBuilder),
    Reclustering(IncrementalBuilder),
    Vectorize(Clusters),
}


#[wasm_bindgen]
pub struct ColorImageConverter {
	debug: bool,
	counter: usize,
	mode: PathSimplifyMode,
	params: ColorImageConverterParams,
	image: ColorImage,
	svg: Svg,
	stage: Stage,
	height: usize,
	width: usize,
}

#[wasm_bindgen]
impl ColorImageConverter {
	#[wasm_bindgen(constructor)]
	// Tsify automatically converts params using serde_wasm_bindgen::from_value(params) where params was JsValue
	pub fn new(
		imageData: ImageData,
		params: ColorImageConverterParams,
		options: Options,
	) -> Self {
		let data = imageData.data();
		let len = data.len();
		let image = ColorImage{
			pixels: imageData.data().to_vec(),
			width: imageData.width() as usize,
			height: imageData.height() as usize,
		};
		let debug = params.debug.is_some_and(|x| x == true);
		if (debug) {
			log(format!("{:#?}", params).as_str());
			log(format!(
				"{:#?}",
				DebugImageData {
					width: image.width,
					first_val: image.get_pixel_safe(0, 0).is_some(),
					height: image.height,
					data_len: len
				}
			)
			.as_str());
		}
		Self {
			debug,
			stage: Stage::New,
			counter: 0,
			mode: path_simplify_mode(&params.mode),
			image,
			params,
			svg: Svg::new(SvgOptions {
				scale: options.scale,
				backgroundColor: options.backgroundColor.clone(),
				pathFill: options.pathFill.clone(),
				attributes: options.attributes.clone(),
			}),
			height: imageData.height() as usize,
			width: imageData.width() as usize,
		}
	}

	pub fn init(&mut self) {
		let runner = Runner::new(RunnerConfig {
            diagonal: self.params.layer_difference == 0,
            hierarchical: HIERARCHICAL_MAX,
            batch_size: 25600,
            good_min_area: self.params.filter_speckle,
            good_max_area: (self.width * self.height) as usize,
            is_same_color_a: self.params.color_precision,
            is_same_color_b: 1,
            deepen_diff: self.params.layer_difference,
            hollow_neighbours: 1,
			key_color: Color::default(),
			keying_action: KeyingAction::default(),
        }, self.image.clone());
        self.stage = Stage::Clustering(runner.start());
	}

	pub fn tick(&mut self) -> bool {
		match &mut self.stage {
            Stage::New => {
                panic!("uninitialized");
            },
            Stage::Clustering(builder) => {
				if (self.debug) {
					log(format!("{:#?}", "Clustering").as_str());
				}
                if builder.tick() {
                    match self.params.hierarchical.as_str() {
                        "stacked" => {
							if (self.debug) {
								log(format!("{:#?}", "stacked").as_str());
							}
                            self.stage = Stage::Vectorize(builder.result());
                        },
                        "cutout" => {
                            let clusters = builder.result();
                            let view = clusters.view();
                            let image = view.to_color_image();
                            let runner = Runner::new(RunnerConfig {
                                diagonal: false,
                                hierarchical: 64,
                                batch_size: 25600,
                                good_min_area: 0,
                                good_max_area: (image.width * image.height) as usize,
                                is_same_color_a: 0,
                                is_same_color_b: 1,
                                deepen_diff: 0,
                                hollow_neighbours: 0,
								..Default::default()
                            }, image);
                            self.stage = Stage::Reclustering(runner.start());
                        },
                        _ => panic!("unknown hierarchical `{}`", self.params.hierarchical)
                    }
                }
                false
            },
            Stage::Reclustering(builder) => {
				if (self.debug) {
					log(format!("{:#?}", "Reclustering").as_str());
				}
                if builder.tick() {
                    self.stage = Stage::Vectorize(builder.result())
                }
                false
            },
            Stage::Vectorize(clusters) => {
				if (self.debug) {
					log(format!("{:#?}", "Vectorize").as_str());
				}
                let view = clusters.view();
                if self.counter < view.clusters_output.len() {
                    let cluster = view.get_cluster(view.clusters_output[self.counter]);
                    let paths = cluster.to_compound_path(
                        &view, false, self.mode,
                        self.params.corner_threshold,
                        self.params.length_threshold,
                        self.params.max_iterations,
                        self.params.splice_threshold
                    );

					if (self.debug) {
						log(format!("{:#?}", paths).as_str());
					}

                    self.svg.prepend_path(
                        &paths,
                        &cluster.residue_color(),
                        Some(self.params.path_precision),
                    );
                    self.counter += 1;
                    false
                } else {
                    true
                }
            }
        }
	}
	pub fn getResult(&self) -> String {
		let result = self.svg.get_svg_string();

		if (self.debug) {
			log(&result.as_str());
		};
		result
	}

	pub fn progress(&self) -> i32 {
        (match &self.stage {
            Stage::New => {
                0
            },
            Stage::Clustering(builder) => {
                builder.progress() / 2
            },
            Stage::Reclustering(_builder) => {
                50
            },
            Stage::Vectorize(clusters) => {
                50 + 50 * self.counter as u32 / clusters.view().clusters_output.len() as u32
            }
        }) as i32
    }
}
