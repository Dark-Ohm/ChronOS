// Stub — implemented in Task 3

pub struct TextStyle {
    pub color: Option<u32>,
    pub size: Option<f32>,
}

pub enum Alignment { Start, Center, End }

pub enum Element {
    Text { content: String, style: TextStyle },
    Row { children: Vec<Element>, gap: f32, alignment: Alignment },
    Column { children: Vec<Element>, gap: f32, alignment: Alignment },
}
