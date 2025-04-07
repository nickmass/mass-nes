pub use lyon::tessellation::{FillVertex, StrokeVertex, VertexBuffers};
pub use usvg::Color;

use lyon::tessellation::*;
use usvg::tiny_skia_path::{PathSegment, Point as PathPoint, Transform as PathTransform};

#[derive(Debug)]
pub enum Error {
    Usvg(usvg::Error),
    Tesselation(TessellationError),
    Unsupported,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Usvg(err) => write!(f, "SVG error: {}", err),
            Error::Tesselation(err) => write!(f, "Tesselation error: {}", err),
            Error::Unsupported => write!(f, "Unsupported SVG element"),
        }
    }
}

impl From<usvg::Error> for Error {
    fn from(error: usvg::Error) -> Self {
        Error::Usvg(error)
    }
}

impl From<TessellationError> for Error {
    fn from(error: TessellationError) -> Self {
        Error::Tesselation(error)
    }
}

pub struct SvgMesh<M: Mesh> {
    pub dimensions: (f32, f32),
    pub geometry: M,
}

pub fn load_svg<M: Mesh>(data: &[u8], tolerance: f32) -> Result<SvgMesh<M>, Error> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(data, &options)?;
    let size = tree.size();
    let dimensions = (size.width(), size.height());
    let mut tesselator = SvgTesselator::<M>::new(tolerance, dimensions);

    tesselator.tesselate_group(tree.root())?;
    let geometry = M::from_geometry(tesselator.geometry);

    let mesh = SvgMesh {
        dimensions,
        geometry,
    };

    Ok(mesh)
}

struct SvgTesselator<M: Mesh> {
    fill_tesselator: FillTessellator,
    stroke_tesselator: StrokeTessellator,
    fill_options: FillOptions,
    stroke_options: StrokeOptions,
    geometry: VertexBuffers<M::Vertex, M::Index>,
    transform: Transform,
    size: (f32, f32),
}

impl<M: Mesh> SvgTesselator<M> {
    fn new(tolerance: f32, size: (f32, f32)) -> Self {
        let fill_tesselator = FillTessellator::new();
        let stroke_tesselator = StrokeTessellator::new();

        let fill_options = FillOptions::default().with_tolerance(tolerance);
        let stroke_options = StrokeOptions::default().with_tolerance(tolerance);

        let geometry = VertexBuffers::new();

        let transform = Transform::new();

        SvgTesselator {
            fill_tesselator,
            stroke_tesselator,
            fill_options,
            stroke_options,
            geometry,
            transform,
            size,
        }
    }

    fn tesselate_group(&mut self, group: &usvg::Group) -> Result<(), Error> {
        for node in group.children() {
            match node {
                usvg::Node::Group(group) => self.tesselate_group(group)?,
                usvg::Node::Path(path) => self.tesselate_path(path)?,
                usvg::Node::Image(_) => return Err(Error::Unsupported),
                usvg::Node::Text(_) => return Err(Error::Unsupported),
            }
        }

        Ok(())
    }

    fn tesselate_path(&mut self, path: &usvg::Path) -> Result<(), Error> {
        use lyon::tessellation::*;
        let lyon_path = build_path(path, &self.transform);
        let id = (!path.id().is_empty()).then_some(path.id());
        if let Some(fill) = path.fill() {
            let usvg::Paint::Color(color) = fill.paint() else {
                return Err(Error::Unsupported);
            };

            self.fill_tesselator.tessellate_path(
                &lyon_path,
                &self.fill_options,
                &mut BuffersBuilder::new(
                    &mut self.geometry,
                    |vertex: lyon::tessellation::FillVertex| {
                        M::Vertex::from_fill_vertex(id, vertex, color, self.size)
                    },
                ),
            )?;
        }
        if let Some(stroke) = path.stroke() {
            let usvg::Paint::Color(color) = stroke.paint() else {
                return Err(Error::Unsupported);
            };

            let line_cap = match stroke.linecap() {
                usvg::LineCap::Butt => lyon::tessellation::LineCap::Butt,
                usvg::LineCap::Round => lyon::tessellation::LineCap::Round,
                usvg::LineCap::Square => lyon::tessellation::LineCap::Square,
            };
            let options = self
                .stroke_options
                .with_line_width(stroke.width().get())
                .with_line_cap(line_cap);

            self.stroke_tesselator.tessellate_path(
                &lyon_path,
                &options,
                &mut BuffersBuilder::new(
                    &mut self.geometry,
                    |vertex: lyon::tessellation::StrokeVertex| {
                        M::Vertex::from_stroke_vertex(id, vertex, color, self.size)
                    },
                ),
            )?;
        }

        Ok(())
    }
}

fn build_path(svg_path: &usvg::Path, transform: &Transform) -> lyon::path::Path {
    let path_transform = svg_path.abs_transform();
    use lyon::geom::point;
    let mut builder = lyon::path::Path::svg_builder();

    for segment in svg_path
        .data()
        .segments()
        .map(|s| transform.transform_segment(&path_transform, s))
    {
        match segment {
            PathSegment::MoveTo(p) => {
                builder.move_to(point(p.x, p.y));
            }
            PathSegment::LineTo(p) => {
                builder.line_to(point(p.x, p.y));
            }
            PathSegment::QuadTo(p0, p1) => {
                builder.quadratic_bezier_to(point(p0.x, p0.y), point(p1.x, p1.y));
            }
            PathSegment::CubicTo(p0, p1, p2) => {
                builder.cubic_bezier_to(point(p0.x, p0.y), point(p1.x, p1.y), point(p2.x, p2.y));
            }
            PathSegment::Close => builder.close(),
        }
    }

    builder.build()
}

struct Transform;

impl Transform {
    fn new() -> Self {
        Transform
    }

    fn transform_segment(
        &self,
        path_transform: &PathTransform,
        segment: PathSegment,
    ) -> PathSegment {
        match segment {
            PathSegment::MoveTo(p0) => {
                PathSegment::MoveTo(self.transform_point(path_transform, p0))
            }
            PathSegment::LineTo(p0) => {
                PathSegment::LineTo(self.transform_point(path_transform, p0))
            }
            PathSegment::QuadTo(p0, p1) => PathSegment::QuadTo(
                self.transform_point(path_transform, p0),
                self.transform_point(path_transform, p1),
            ),
            PathSegment::CubicTo(p0, p1, p2) => PathSegment::CubicTo(
                self.transform_point(path_transform, p0),
                self.transform_point(path_transform, p1),
                self.transform_point(path_transform, p2),
            ),
            PathSegment::Close => PathSegment::Close,
        }
    }

    fn transform_point(
        &self,
        segment_transform: &PathTransform,
        mut point: PathPoint,
    ) -> PathPoint {
        segment_transform.map_point(&mut point);
        point
    }
}

pub trait Mesh {
    type Index: std::ops::Add + From<VertexId> + geometry_builder::MaxIndex;
    type Vertex: Vertex;
    fn from_geometry(geometry: VertexBuffers<Self::Vertex, Self::Index>) -> Self;
}

pub trait Vertex {
    fn from_stroke_vertex(
        id: Option<&str>,
        vertex: StrokeVertex,
        color: &usvg::Color,
        size: (f32, f32),
    ) -> Self;
    fn from_fill_vertex(
        id: Option<&str>,
        vertex: FillVertex,
        color: &usvg::Color,
        size: (f32, f32),
    ) -> Self;
}

impl Mesh for epaint::Mesh {
    type Index = u32;
    type Vertex = epaint::Vertex;
    fn from_geometry(geometry: VertexBuffers<epaint::Vertex, Self::Index>) -> Self {
        let VertexBuffers { vertices, indices } = geometry;

        Self {
            indices,
            vertices,
            texture_id: epaint::TextureId::Managed(0),
        }
    }
}

impl Mesh for epaint::Mesh16 {
    type Index = u16;
    type Vertex = epaint::Vertex;
    fn from_geometry(geometry: VertexBuffers<epaint::Vertex, Self::Index>) -> Self {
        let VertexBuffers { vertices, indices } = geometry;

        Self {
            indices,
            vertices,
            texture_id: epaint::TextureId::Managed(0),
        }
    }
}

impl Vertex for epaint::Vertex {
    fn from_stroke_vertex(
        _id: Option<&str>,
        vertex: StrokeVertex,
        color: &usvg::Color,
        size: (f32, f32),
    ) -> Self {
        let pos = vertex.position();
        epaint::Vertex {
            pos: epaint::pos2(pos.x / size.0, pos.y / size.1),
            uv: epaint::WHITE_UV,
            color: epaint::Color32::from_rgb(color.red, color.green, color.blue),
        }
    }

    fn from_fill_vertex(
        _id: Option<&str>,
        vertex: FillVertex,
        color: &usvg::Color,
        size: (f32, f32),
    ) -> Self {
        let pos = vertex.position();
        epaint::Vertex {
            pos: epaint::pos2(pos.x / size.0, pos.y / size.1),
            uv: epaint::WHITE_UV,
            color: epaint::Color32::from_rgb(color.red, color.green, color.blue),
        }
    }
}
