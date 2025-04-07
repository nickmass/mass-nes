use eframe::{egui, egui_glow};

use std::sync::{Arc, Mutex};

use svg_tessellator::SvgMesh;

use crate::gl;

pub struct SvgGlView {
    svg: Arc<Mutex<SvgRender>>,
    size: egui::Vec2,
    scale: f32,
}

impl SvgGlView {
    fn new(svg: SvgMesh<GlSvgMesh>) -> Self {
        let size = svg.dimensions.into();
        let svg = Arc::new(Mutex::new(SvgRender::Unloaded(svg)));
        Self {
            svg,
            size,
            scale: 1.0,
        }
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn size(&self) -> egui::Vec2 {
        self.size * self.scale
    }

    pub fn view<H: Into<u32>>(
        &self,
        renderer: &SvgRenderer,
        highlight: H,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        let (res, painter) = ui.allocate_painter(self.size(), egui::Sense::empty());
        let renderer = renderer.inner.clone();
        let svg = self.svg.clone();
        let c = ui.visuals().selection.stroke.color.to_tuple();
        let highlight_color = (
            c.0 as f32 / 255.0,
            c.1 as f32 / 255.0,
            c.2 as f32 / 255.0,
            c.3 as f32 / 255.0,
        );
        let highlight = highlight.into();
        let callback = egui::PaintCallback {
            rect: res.rect,
            callback: Arc::new(egui_glow::CallbackFn::new(move |paint_info, painter| {
                let Ok(mut renderer) = renderer.lock() else {
                    return;
                };
                renderer.render(highlight, highlight_color, &svg, painter, paint_info);
            })),
        };

        painter.add(callback);

        res
    }
}

struct GlSvgMesh {
    vertcies: Vec<GlSvgVertex>,
    indices: Vec<u16>,
}

#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct GlSvgVertex {
    highlight_mask: u32,
    position: [f32; 2],
    color: [f32; 4],
}

impl svg_tessellator::Mesh for GlSvgMesh {
    type Index = u16;

    type Vertex = GlSvgVertex;

    fn from_geometry(geometry: svg_tessellator::VertexBuffers<Self::Vertex, Self::Index>) -> Self {
        Self {
            vertcies: geometry.vertices,
            indices: geometry.indices,
        }
    }
}

impl svg_tessellator::Vertex for GlSvgVertex {
    fn from_stroke_vertex(
        id: Option<&str>,
        vertex: svg_tessellator::StrokeVertex,
        color: &svg_tessellator::Color,
        size: (f32, f32),
    ) -> Self {
        let pos = vertex.position();
        let highlight_mask = id
            .and_then(|id| u32::from_str_radix(id, 16).ok())
            .unwrap_or(0);
        Self {
            highlight_mask,
            position: [pos.x / size.0, pos.y / size.1],
            color: [
                color.red as f32 / 255.0,
                color.green as f32 / 255.0,
                color.blue as f32 / 255.0,
                1.0,
            ],
        }
    }

    fn from_fill_vertex(
        id: Option<&str>,
        vertex: svg_tessellator::FillVertex,
        color: &svg_tessellator::Color,
        size: (f32, f32),
    ) -> Self {
        let pos = vertex.position();
        let highlight_mask = id
            .and_then(|id| u32::from_str_radix(id, 16).ok())
            .unwrap_or(0);
        Self {
            highlight_mask,
            position: [pos.x / size.0, pos.y / size.1],
            color: [
                color.red as f32 / 255.0,
                color.green as f32 / 255.0,
                color.blue as f32 / 255.0,
                1.0,
            ],
        }
    }
}

impl gl::Vertex for GlSvgVertex {
    const ATTRIBUTES: &[(&'static str, gl::AttrType)] = &[
        ("highlight_mask", gl::AttrType::U32),
        ("position", gl::AttrType::Vec2),
        ("color", gl::AttrType::Vec4),
    ];

    const SIZE: usize = 24;
}

enum SvgRender {
    Unloaded(SvgMesh<GlSvgMesh>),
    Loaded {
        vertex_buffer: gl::VertexBuffer<GlSvgVertex>,
        index_buffer: gl::IndexBuffer,
    },
    Lost,
}
pub struct SvgRenderer {
    inner: Arc<Mutex<SvgRendererInner>>,
}

impl SvgRenderer {
    pub fn new(ctx: gl::GlowContext) -> Result<Self, ()> {
        let inner = SvgRendererInner::new(ctx)?;

        Ok(Self {
            inner: Arc::new(Mutex::new(inner)),
        })
    }
}

struct SvgRendererInner {
    program: gl::Program,
}

impl SvgRendererInner {
    fn new(ctx: gl::GlowContext) -> Result<Self, ()> {
        let shaders_source = ui::filters::Preprocessor::new(ui::filters::SVG_SHADER);
        let program = shaders_source.process().map_err(|_| ())?;
        let program = gl::Program::new(&ctx, program.vertex, program.fragment).map_err(|_| ())?;

        Ok(Self { program })
    }

    fn render(
        &mut self,
        highlight: u32,
        highlight_color: (f32, f32, f32, f32),
        svg: &Arc<Mutex<SvgRender>>,
        painter: &egui_glow::Painter,
        _paint_info: egui::epaint::PaintCallbackInfo,
    ) {
        let Ok(mut svg) = svg.lock() else {
            return;
        };
        let ctx = painter.gl().clone();

        match &mut *svg {
            SvgRender::Unloaded(svg_mesh) => {
                let vertex_buffer = gl::VertexBuffer::new(
                    &ctx,
                    gl::Polygon::Triangles,
                    svg_mesh.geometry.vertcies.as_slice(),
                )
                .ok();

                let index_buffer =
                    gl::IndexBuffer::new(&ctx, svg_mesh.geometry.indices.as_slice()).ok();
                *svg = if let Some((vertex_buffer, index_buffer)) = vertex_buffer.zip(index_buffer)
                {
                    SvgRender::Loaded {
                        vertex_buffer,
                        index_buffer,
                    }
                } else {
                    SvgRender::Lost
                };
            }
            SvgRender::Loaded {
                vertex_buffer,
                index_buffer,
            } => {
                let mut uniforms = gl::Uniforms::new(&ctx);
                uniforms.add("highlight", highlight);
                uniforms.add("highlight_color", highlight_color);
                self.program
                    .draw(&ctx, &vertex_buffer, Some(&index_buffer), &uniforms);
            }
            SvgRender::Lost => (),
        }
    }
}

pub fn nes_controller() -> SvgGlView {
    let mesh = load_nes_controller();

    SvgGlView::new(mesh)
}

fn load_nes_controller() -> SvgMesh<GlSvgMesh> {
    svg_tessellator::load_svg(NES_CONTROLLER_SVG.as_bytes(), 0.5).unwrap()
}

#[test]
fn test_embedded_svgs() {
    load_nes_controller();
}

use bitflags::bitflags;

bitflags! {
    pub struct NesButtons: u32 {
        const UP =      0x01;
        const DOWN =    0x02;
        const LEFT =    0x04;
        const RIGHT =   0x08;
        const A =       0x10;
        const B =       0x20;
        const SELECT =  0x40;
        const START =   0x80;
    }
}

const NES_CONTROLLER_SVG: &str = r##"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
<!-- Attribution: I, Fant0men -->
<!-- License:  Creative Commons Attribution-Share Alike 3.0 Unported license (https://creativecommons.org/licenses/by-sa/3.0/deed.en) -->
<!-- Source: https://commons.wikimedia.org/wiki/File:Nes_controller.svg (6 April 2025) -->
<svg version="1.1"
	xmlns="http://www.w3.org/2000/svg" x="0px" y="0px" width="612.002px" height="251.924px"
	viewBox="-0.004 270.034 612.002 251.924" xml:space="preserve">
	<g>
		<rect x="15.947" y="311.988" fill="#1A1A1A" width="581.833" height="195.284"/>
		<path fill="#808080" d="M205.958,305.043h153.281c5.362,0,9.709,4.347,9.709,9.709v10.594
			c0,5.362-4.347,9.709-9.709,9.709H205.958c-5.362,0-9.709-4.347-9.709-9.709v-10.594
			C196.25,309.39,200.597,305.043,205.958,305.043z"/>
		<path fill="#808080" d="M205.958,488.063h153.281c5.362,0,9.709,4.347,9.709,9.709v10.594
			c0,5.362-4.347,9.709-9.709,9.709H205.958c-5.362,0-9.709-4.347-9.709-9.709v-10.594
			C196.25,492.409,200.597,488.063,205.958,488.063z"/>
		<path fill="#DCDCDC" d="M8.147,270.034c-4.508,0-8.152,3.644-8.152,8.152v235.621
			c0,4.508,3.645,8.152,8.152,8.151h595.897c4.508,0,7.953-3.644,7.953-8.151V278.186c0-4.508-3.445-8.152-7.953-8.152H8.147
			L8.147,270.034z M15.902,311.988h581.986v195.25H15.902V311.988L15.902,311.988z"/>
		<path stroke="#FFFFFF" stroke-width="1.6523" stroke-linecap="round" d="M87.542,362.004
			c-2.686,0-4.713,2.216-4.713,4.902v33.747H49.082c-2.686,0-4.902,2.217-4.902,4.902v27.148c0,2.686,2.216,4.901,4.902,4.901
			h33.747v33.747c0,2.686,2.028,4.714,4.713,4.714h27.337c2.686,0,4.713-2.027,4.713-4.714v-33.747h33.747
			c2.686,0,4.902-2.216,4.902-4.901v-27.148c0-2.686-2.216-4.902-4.902-4.902h-33.747v-33.747c0-2.686-2.028-4.902-4.713-4.902
			H87.542z"/>
		<path fill="#1A1A1A" d="M88.484,366.254c-0.85,0-1.506,0.657-1.506,1.506v36.832H50.146
			c-0.849,0-1.506,0.656-1.506,1.506v25.948c0,0.85,0.657,1.574,1.506,1.574h36.832v36.765c0,0.849,0.657,1.574,1.506,1.574H114.5
			c0.85,0,1.506-0.726,1.506-1.574V433.62h36.832c0.85,0,1.506-0.725,1.506-1.574v-25.948c0-0.85-0.657-1.506-1.506-1.506h-36.832
			V367.76c0-0.85-0.656-1.506-1.506-1.506H88.484z"/>
		<path fill="#808080" d="M205.958,343.667h153.281c5.362,0,9.709,4.347,9.709,9.708v10.594
			c0,5.362-4.347,9.709-9.709,9.709H205.958c-5.362,0-9.709-4.347-9.709-9.709v-10.594
			C196.25,348.014,200.597,343.667,205.958,343.667z"/>
		<g transform="matrix(2.9042238,0,0,2.9042238,-597.29495,-994.37335)">
			<path fill="#DDDDDD" d="M343.472,484.842h20.578c0.908,0,1.643,0.736,1.643,1.643v20.578
				c0,0.907-0.735,1.643-1.643,1.643h-20.578c-0.908,0-1.643-0.736-1.643-1.643v-20.578
				C341.829,485.577,342.565,484.842,343.472,484.842z"/>
			<path id="20" fill="#FF0000" d="M363.54,496.875c0,5.402-4.379,9.78-9.78,9.78s-9.78-4.379-9.78-9.78
				s4.379-9.78,9.78-9.78S363.54,491.473,363.54,496.875z"/>
		</g>
		<g transform="matrix(2.9042238,0,0,2.9042238,-597.29495,-994.37335)">
			<path fill="#DDDDDD" d="M370.572,484.842h20.578c0.907,0,1.643,0.736,1.643,1.643v20.578
				c0,0.907-0.736,1.643-1.643,1.643h-20.578c-0.908,0-1.643-0.736-1.643-1.643v-20.578
				C368.929,485.577,369.665,484.842,370.572,484.842z"/>
			<path id="10" fill="#FF0000" d="M390.65,496.875c0,5.402-4.379,9.78-9.78,9.78c-5.402,0-9.781-4.379-9.781-9.78
				s4.379-9.78,9.781-9.78C386.271,487.094,390.65,491.473,390.65,496.875z"/>
		</g>
		<path fill="#808080" d="M205.958,382.312h153.281c5.362,0,9.709,4.347,9.709,9.709v10.594
			c0,5.362-4.347,9.709-9.709,9.709H205.958c-5.362,0-9.709-4.347-9.709-9.709v-10.594
			C196.249,386.659,200.597,382.312,205.958,382.312z"/>
		<path fill="#DDDDDD" d="M207.071,420.454h151.087c5.976,0,10.82,4.845,10.82,10.821v35.343
			c0,5.977-4.845,10.821-10.82,10.821H207.071c-5.977,0-10.821-4.845-10.821-10.821v-35.343
			C196.249,425.299,201.094,420.454,207.071,420.454z"/>
		<path fill="none" stroke="#808080" stroke-width="1.4584" stroke-linecap="round" d="M212.19,425.887h140.876
			c4.896,0,8.864,3.969,8.864,8.864v28.951c0,4.896-3.969,8.863-8.864,8.863H212.19c-4.896,0-8.864-3.968-8.864-8.863v-28.951
			C203.326,429.856,207.294,425.887,212.19,425.887z"/>
		<path d="M225.68,440.412h29.395c5.28,0,9.56,4.28,9.56,9.561l0,0c0,5.279-4.28,9.56-9.56,9.56H225.68
			c-5.28,0-9.56-4.28-9.56-9.56l0,0C216.12,444.692,220.4,440.412,225.68,440.412z"/>
		<path d="M309.676,440.412h29.395c5.28,0,9.561,4.28,9.561,9.561l0,0c0,5.279-4.28,9.56-9.561,9.56h-29.395
			c-5.28,0-9.561-4.28-9.561-9.56l0,0C300.115,444.692,304.395,440.412,309.676,440.412z"/>
		<!--
		<g>
			<path fill="#FE0016" d="M429.575,346.888v15.725h-5.008l-6.401-10.653c-0.354-0.699-0.354-0.014-0.354-0.014
				v10.667h-5.125v-15.725h5.125c0,0,6.106,10.071,6.44,10.544c0.314,0.474,0.314,0,0.314,0v-10.544H429.575"/>
			<path fill="#FE0016" d="M432.127,349.752h4.692v-2.865h-4.692V349.752z"/>
			<path fill="#FE0016" d="M461.759,349.205v1.705h2.101l0.079,1.037h-2.18v10.667h-4.89v-10.667h-2.199l0.06-1.037
				h2.14v-1.705H461.759"/>
			<path fill="#FE0016" d="M452.47,352.47c1.178,0.938,1.59,2.751,1.59,2.751v7.391h-4.929v-7.267
				c0,0-0.373-1.716-2.297-1.705c-1.904,0.012-2.277,1.581-2.277,1.581v7.391h-4.812v-10.666h4.812
				c0.49,0.915,1.649-0.183,2.65-0.354C449.033,351.008,451.017,351.593,452.47,352.47"/>
			<path fill="#FE0016" d="M432.127,362.612h4.692v-10.667h-4.692V362.612z"/>
			<path fill="#FE0016" d="M473.089,360.845c0,0-0.433,1.463-1.983,1.463c-1.552,0-1.905-1.646-1.905-1.646
				l0.06-3.232h8.935c-0.021-2.985-1.276-4.143-2.298-4.876c-3.004-1.646-7.266-1.62-10.191,0.719
				c-1.021,0.878-1.747,2.488-1.669,4.023c0,1.39,0.569,2.778,1.532,3.731c1.904,1.608,4.752,2.338,7.54,1.755
				c1.826-0.585,4.163-1.463,4.673-3.511h-4.673L473.089,360.845L473.089,360.845z M469.357,354.088c0,0,0.314-1.412,1.904-1.412
				c1.571,0,1.768,1.549,1.768,1.549l0.039,1.438h-3.711V354.088"/>
			<path fill="#FE0016" d="M504.899,346.886v5.471c-1.61-0.878-4.301-1.424-5.989-0.326
				c-2.121,1.023-2.848,3.438-2.926,5.559c0.078,1.608,1.021,3.436,2.415,4.389c1.748,1.068,4.084,1.221,6.048,0.404
				c0.177-0.079,0.373-0.163,0.53-0.259c0.157,0.122,0.354,0.218,0.569,0.295c1.395,0.483,3.868,0.194,3.868,0.194v-15.727H504.899
				L504.899,346.886z M504.899,360.708c0,0-0.491,1.039-1.886,1.051c-1.1-0.012-1.846-0.792-1.846-0.792v-7.01
				c0,0,0.314-0.988,1.729-0.974c1.414,0.01,2.003,0.974,2.003,0.974V360.708"/>
			<path fill="#FE0016" d="M492.92,352.512c1.159,0.938,1.591,2.751,1.591,2.751v7.391h-4.948v-7.268
				c0,0-0.373-1.716-2.278-1.704c-1.924,0.014-2.277,1.581-2.277,1.581v7.391h-4.831v-10.667h4.831
				c0.471,0.915,1.63-0.183,2.631-0.352C489.464,351.049,491.447,351.636,492.92,352.512"/>
			<path fill="#FE0016" d="M518.625,351.276c-3.967,0-7.167,2.606-7.167,5.82c0,3.212,3.2,5.818,7.167,5.818
				c3.946-0.002,7.147-2.606,7.147-5.818C525.772,353.882,522.572,351.276,518.625,351.276z M520.549,360.56
				c0,0-0.49,1.039-1.904,1.05c-1.08-0.012-1.826-0.791-1.826-0.791v-7.01c0,0,0.295-0.988,1.708-0.974
				c1.414,0.012,2.022,0.974,2.022,0.974V360.56"/>
			<path fill="#FE0016" d="M527.859,352.205c1.039,0,2.021-0.411,2.765-1.148c0.741-0.74,1.159-1.721,1.159-2.764
				c0-1.043-0.418-2.024-1.159-2.764c-0.744-0.737-1.726-1.148-2.765-1.148c-2.17,0-3.924,1.754-3.924,3.912
				c0,1.043,0.417,2.024,1.159,2.764C525.838,351.795,526.818,352.205,527.859,352.205L527.859,352.205z M524.708,348.293
				c0-0.838,0.328-1.625,0.922-2.217c0.595-0.591,1.397-0.918,2.229-0.918s1.635,0.327,2.229,0.918
				c0.564,0.592,0.892,1.379,0.892,2.217c0,1.73-1.397,3.136-3.121,3.136c-0.832,0-1.635-0.327-2.229-0.918
				C525.036,349.919,524.708,349.131,524.708,348.293"/>
			<path fill="#FE0016" d="M527.025,346.697h1.338c0.178,0,0.327,0.027,0.444,0.077
				c0.18,0.104,0.299,0.297,0.299,0.579c0,0.262-0.089,0.447-0.237,0.547c-0.149,0.098-0.328,0.152-0.596,0.152h-1.248V346.697
				L527.025,346.697z M527.025,350.204v-1.703h1.248c0.208,0,0.388,0.021,0.476,0.071c0.18,0.089,0.269,0.265,0.297,0.523
				l0.03,0.66c0,0.158,0,0.262,0.03,0.315c0,0.054,0,0.098,0.03,0.134h0.653v-0.092c-0.089-0.036-0.118-0.115-0.149-0.24
				c-0.03-0.075-0.059-0.188-0.059-0.339l-0.03-0.532c0-0.229-0.029-0.397-0.118-0.499c-0.06-0.104-0.179-0.188-0.357-0.25
				c0.179-0.092,0.327-0.214,0.417-0.362c0.117-0.155,0.147-0.351,0.147-0.585c0-0.458-0.178-0.77-0.564-0.939
				c-0.178-0.089-0.445-0.131-0.742-0.131h-1.843v3.971H527.025"/>
		</g>
		-->
		<g id="text_select">
			<path fill="#FF0000" d="M205.847,403.028h7.825c2.236,0,3.354-1.025,3.354-3.075c0-2.049-1.118-3.074-3.354-3.074
				h-5.589v-1.676h8.943v-2.236h-7.825c-2.236,0-3.354,1.039-3.354,3.118c0,2.02,1.118,3.03,3.354,3.03h5.589v1.677h-8.943V403.028
				z"/>
			<path fill="#FF0000" d="M229.324,403.028h-7.825c-2.235,0-3.354-1.118-3.354-3.354v-6.707h11.179v2.235h-8.943
				v1.676h8.943v2.235h-8.943v0.56c0,0.745,0.373,1.118,1.118,1.118h7.825V403.028z"/>
			<path fill="#FF0000" d="M230.427,392.954v6.707c0,2.235,1.118,3.354,3.353,3.354h7.826v-2.235h-7.826
				c-0.745,0-1.118-0.373-1.118-1.118v-6.707H230.427z"/>
			<path fill="#FF0000" d="M253.903,403.028h-7.825c-2.236,0-3.353-1.118-3.353-3.354v-6.707h11.178v2.235h-8.943
				v1.676h8.943v2.235h-8.943v0.56c0,0.745,0.373,1.118,1.118,1.118h7.825V403.028z"/>
			<path fill="#FF0000" d="M266.204,403.028h-7.825c-2.236,0-3.354-1.118-3.354-3.354v-3.354
				c0-2.235,1.118-3.353,3.354-3.353h7.825v2.235h-7.825c-0.746,0-1.118,0.373-1.118,1.118v3.354c0,0.745,0.373,1.118,1.118,1.118
				h7.825V403.028z"/>
			<path fill="#FF0000" d="M267.328,392.954h11.179v2.236h-4.472v7.825h-2.235v-7.825h-4.472V392.954z"/>
		</g>
		<g id="text_start">
			<path fill="#FF0000" d="M298.659,403.028h7.825c2.236,0,3.352-1.025,3.352-3.075c0-2.049-1.116-3.074-3.352-3.074
				h-5.59v-1.676h8.942v-2.236h-7.824c-2.236,0-3.354,1.039-3.354,3.119c0,2.02,1.118,3.029,3.354,3.029h5.589v1.677h-8.943
				V403.028z"/>
			<path fill="#FF0000" d="M310.937,392.954h11.18v2.236h-4.472v7.825h-2.236v-7.825h-4.472V392.954z"/>
			<path fill="#FF0000" d="M332.187,397.991v-1.677c0-0.746-0.373-1.119-1.118-1.119h-4.471
				c-0.746,0-1.118,0.373-1.118,1.119v1.677H332.187z M325.48,400.226v2.795h-2.235v-6.707c0-2.236,1.118-3.354,3.354-3.354h4.471
				c2.236,0,3.354,1.118,3.354,3.354v6.707h-2.236v-2.795H325.48z"/>
			<path fill="#FF0000" d="M337.765,396.867h6.708v-1.677h-6.708V396.867z M337.765,399.102v3.912h-2.234v-10.06
				h7.824c2.236,0,3.354,1.021,3.354,3.063c0,0.969-0.16,1.681-0.48,2.135c0.32,0.529,0.48,1.219,0.48,2.068v2.794h-2.235v-2.794
				c0-0.745-0.373-1.118-1.118-1.118H337.765z"/>
			<path fill="#FF0000" d="M347.839,392.954h11.179v2.236h-4.472v7.825h-2.235v-7.825h-4.472V392.954z"/>
		</g>
		<g>
			<path id="text_b" fill="#FF0000" d="M457.062,493.145h8.023v-2.006h-8.023V493.145z M457.062,497.825h8.023v-2.006h-8.023
				V497.825z M454.389,500.499v-12.034h9.36c2.674,0,4.011,1.212,4.011,3.637c0,0.99-0.214,1.783-0.642,2.381
				c0.428,0.588,0.642,1.377,0.642,2.366c0,2.434-1.337,3.65-4.011,3.65H454.389z"/>
		</g>
		<g>
			<path id="text_a" fill="#FF0000" d="M543.818,494.985v-2.006c0-0.892-0.445-1.337-1.337-1.337h-5.349
				c-0.892,0-1.337,0.445-1.337,1.337v2.006H543.818z M535.796,497.659v3.344h-2.675v-8.023c0-2.674,1.338-4.011,4.012-4.011h5.349
				c2.674,0,4.011,1.337,4.011,4.011v8.023h-2.674v-3.344H535.796z"/>
		</g>
		<path id="04" fill="none" stroke="#000000" stroke-width="2.0" d="M63.181,408.339l-5.477,5.478l-5.477,5.477l5.477,5.478
			l5.477,5.477v-3.286h10.954v-15.336H63.181V408.339z"/>
		<path id="08" fill="none" stroke="#000000" stroke-width="2.0" d="M139.683,408.339l5.477,5.478l5.477,5.477l-5.477,5.478
			l-5.477,5.477v-3.286h-10.954v-15.336h10.954V408.339z"/>
		<path id="01" fill="none" stroke="#000000" stroke-width="2.0" d="M112.276,380.669l-5.477-5.477l-5.477-5.477
			l-5.477,5.477l-5.477,5.477h3.286v10.954h15.336v-10.954H112.276z"/>
		<path id="02" fill="none" stroke="#000000" stroke-width="2.0" d="M112.276,458.333l-5.477,5.478l-5.477,5.477
			l-5.477-5.477l-5.477-5.478h3.286v-10.954h15.336v10.954H112.276z"/>
		<path fill="none" stroke="#000000" stroke-width="0.7366" d="M114.646,419.111
			c0.004,7.26-5.876,13.148-13.136,13.153s-13.148-5.877-13.153-13.136c0-0.006,0-0.012,0-0.018
			c-0.005-7.26,5.876-13.148,13.136-13.153c7.259-0.005,13.148,5.876,13.153,13.136
			C114.646,419.099,114.646,419.105,114.646,419.111z"/>
		<path id="40" fill="#1A1A1A" d="M228.2,442.274h24.391c5.28,0,9.56,3.448,9.56,7.702l0,0c0,4.254-4.28,7.702-9.56,7.702
			H228.2c-5.28,0-9.56-3.448-9.56-7.702l0,0C218.64,445.722,222.92,442.274,228.2,442.274z"/>
		<path id="80" fill="#1A1A1A" d="M312.173,442.274h24.392c5.279,0,9.56,3.448,9.56,7.702l0,0c0,4.254-4.28,7.702-9.56,7.702
			h-24.392c-5.279,0-9.56-3.448-9.56-7.702l0,0C302.613,445.722,306.893,442.274,312.173,442.274z"/>
	</g>
</svg>
"##;
