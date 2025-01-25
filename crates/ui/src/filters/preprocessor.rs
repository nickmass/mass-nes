use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Error {
    InvalidDefine(usize),
    InvalidParam(usize),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Preprocessor<'a> {
    source: &'a str,
    defines: HashMap<String, String>,
}

impl<'a> Preprocessor<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            defines: HashMap::new(),
        }
    }

    pub fn add_define<N: Into<String>, V: Into<String>>(&mut self, name: N, value: V) {
        let name = name.into();
        let value = value.into();

        self.defines.insert(name, value);
    }

    pub fn process(self) -> Result<Program<'a>, Error> {
        let mut global = String::from(super::PRELUDE_SHADER);
        let mut vertex = String::new();
        let mut fragment = String::new();

        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        enum Stage {
            Global,
            Vertex,
            Fragment,
        }

        let defines = self.defines;
        let mut local_defines = HashMap::new();

        let mut params = Vec::new();

        let mut stage = Stage::Global;

        for (line_num, line) in self.source.lines().enumerate() {
            if line.starts_with("#pragma stage vertex") {
                stage = Stage::Vertex;
            } else if line.starts_with("#pragma stage fragment") {
                stage = Stage::Fragment;
            } else if line.starts_with("#define") {
                let mut parts = line.split_whitespace();
                let _ = parts.next();
                let name = parts.next().ok_or(Error::InvalidDefine(line_num))?;
                let value = parts.next().ok_or(Error::InvalidDefine(line_num))?;
                local_defines.insert(name, value);
            } else if line.starts_with("#pragma parameter") {
                let mut parts = line.split_whitespace();
                let _pragma = parts.next();
                let _param = parts.next();
                let name = parts.next().ok_or(Error::InvalidParam(line_num))?;
                let desc = parts.next().ok_or(Error::InvalidParam(line_num))?;
                let mut desc = String::from(desc);
                if desc.starts_with('"') {
                    while desc.len() == 1 || !desc.ends_with('"') {
                        let next = parts.next().ok_or(Error::InvalidParam(line_num))?;
                        desc.push(' ');
                        desc.push_str(next);
                    }

                    desc = desc.trim_matches('"').to_string();
                }

                let value = parts
                    .next()
                    .and_then(|p| p.parse::<f32>().ok())
                    .ok_or(Error::InvalidParam(line_num))?;

                let min = parts
                    .next()
                    .and_then(|p| p.parse::<f32>().ok())
                    .ok_or(Error::InvalidParam(line_num))?;

                let max = parts
                    .next()
                    .and_then(|p| p.parse::<f32>().ok())
                    .ok_or(Error::InvalidParam(line_num))?;

                let step = parts
                    .next()
                    .and_then(|p| p.parse::<f32>().ok())
                    .ok_or(Error::InvalidParam(line_num))?;

                let param = Parameter {
                    name,
                    description: desc,
                    value,
                    min,
                    max,
                    step,
                };

                params.push(param);
            } else {
                let destination = match stage {
                    Stage::Global => &mut global,
                    Stage::Vertex => &mut vertex,
                    Stage::Fragment => &mut fragment,
                };

                let mut line = String::from(line);
                for (name, value) in defines.iter() {
                    line = line.replace(name, value);
                }

                for (name, value) in local_defines.iter() {
                    line = line.replace(name, value);
                }

                destination.push_str(&line);
                destination.push('\n');
            }
        }

        let vertex = format!("{global}{vertex}");
        let fragment = format!("{global}{fragment}");

        let program = Program {
            vertex,
            fragment,
            parameters: params,
        };

        Ok(program)
    }
}

#[derive(Debug, Clone)]
pub struct Program<'a> {
    pub vertex: String,
    pub fragment: String,
    pub parameters: Vec<Parameter<'a>>,
}

#[derive(Debug, Clone)]
pub struct Parameter<'a> {
    pub name: &'a str,
    pub description: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
}
