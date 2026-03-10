use super::*;

#[derive(Boilerplate)]
pub(crate) struct HomeHtml {
    pub(crate) stratum_url: String,
}

impl PageContent for HomeHtml {
    fn title(&self) -> String {
        "Black Pool".to_string()
    }
}
