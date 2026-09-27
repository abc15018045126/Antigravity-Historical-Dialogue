#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Language {
    Zh,
    En,
}

impl Language {
    pub fn default_lang() -> Self {
        if cfg!(debug_assertions) {
            Language::Zh
        } else {
            Language::En
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            Language::Zh => Language::En,
            Language::En => Language::Zh,
        }
    }

    pub fn is_en(&self) -> bool {
        *self == Language::En
    }

    pub fn button_label(&self) -> &'static str {
        match self {
            Language::Zh => "🌐 中文",
            Language::En => "🌐 EN",
        }
    }

    #[allow(dead_code)]
    pub fn t<'a>(&self, zh: &'a str, en: &'a str) -> &'a str {
        if self.is_en() {
            en
        } else {
            zh
        }
    }
}
