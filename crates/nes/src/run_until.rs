pub trait RunUntil {
    fn add_cycle(&mut self) {}

    fn add_instruction(&mut self) {}

    fn add_sample(&mut self) {}

    fn add_dot(&mut self) {}

    fn add_scanline(&mut self) {}

    fn add_frame(&mut self) {}

    fn done(&self) -> bool;

    fn or<T: RunUntil>(self, other: T) -> (Self, T)
    where
        Self: Sized,
    {
        (self, other)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Cycles(pub u32);
impl RunUntil for Cycles {
    fn add_cycle(&mut self) {
        self.0 = self.0.saturating_sub(1);
    }

    fn done(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Instructions(pub u32);
impl RunUntil for Instructions {
    fn add_instruction(&mut self) {
        self.0 = self.0.saturating_sub(1);
    }

    fn done(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Samples(pub u32);
impl RunUntil for Samples {
    fn add_sample(&mut self) {
        self.0 = self.0.saturating_sub(1);
    }

    fn done(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Dots(pub u32);
impl RunUntil for Dots {
    fn add_dot(&mut self) {
        self.0 = self.0.saturating_sub(1);
    }

    fn done(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Scanlines(pub u32);
impl RunUntil for Scanlines {
    fn add_scanline(&mut self) {
        self.0 = self.0.saturating_sub(1);
    }

    fn done(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Frames(pub u32);
impl RunUntil for Frames {
    fn add_frame(&mut self) {
        self.0 = self.0.saturating_sub(1);
    }

    fn done(&self) -> bool {
        self.0 == 0
    }
}

impl<T: RunUntil + ?Sized> RunUntil for Box<T> {
    fn done(&self) -> bool {
        (**self).done()
    }

    fn add_cycle(&mut self) {
        (**self).add_cycle();
    }

    fn add_sample(&mut self) {
        (**self).add_sample();
    }

    fn add_dot(&mut self) {
        (**self).add_dot();
    }

    fn add_scanline(&mut self) {
        (**self).add_scanline();
    }

    fn add_frame(&mut self) {
        (**self).add_frame();
    }
}

impl<T: RunUntil, U: RunUntil> RunUntil for (T, U) {
    fn add_cycle(&mut self) {
        self.0.add_cycle();
        self.1.add_cycle();
    }

    fn add_sample(&mut self) {
        self.0.add_sample();
        self.1.add_sample();
    }

    fn add_dot(&mut self) {
        self.0.add_dot();
        self.1.add_dot();
    }

    fn add_scanline(&mut self) {
        self.0.add_scanline();
        self.1.add_scanline();
    }

    fn add_frame(&mut self) {
        self.0.add_frame();
        self.1.add_frame();
    }

    fn done(&self) -> bool {
        self.0.done() || self.1.done()
    }
}
