pub(crate) trait TesseractEngine: Send + Sync {
    fn available_languages(&self) -> crate::Result<Vec<String>>;

    fn recognize(&self, image_data: &[u8], language: Option<&str>) -> crate::Result<String>;
}
