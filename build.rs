fn main() {
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/papericon.ico");
        let _ = resource.compile();
    }
}
