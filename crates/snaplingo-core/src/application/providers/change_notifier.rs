pub trait ProviderChangeNotifier: Send + Sync {
    fn providers_changed(&self);
}
