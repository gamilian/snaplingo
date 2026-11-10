use snaplingo_ui::render::{Renderer, RendererTextMetrics};
use snaplingo_ui::{notices::*, Appearance, Size};
fn main() -> Result<(), String> {
    let directory = "target/shared-notices";
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let mut ui=NoticesUi::new(Appearance::default(),vec![NoticeComponent {name:"snaplingo-ui".into(),version:"0.1.23".into(),license:"MIT".into(),source:"https://github.com/gamilian/snaplingo".into(),text:"MIT License\n\nCopyright (c) 2026 SnapLingo contributors\n\nPermission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the Software), to deal in the Software without restriction.\n\nTHE SOFTWARE IS PROVIDED AS IS, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED.".into()},NoticeComponent{name:"winit".into(),version:"0.30.13".into(),license:"Apache-2.0".into(),source:"https://github.com/rust-windowing/winit".into(),text:"Apache License\nVersion 2.0, January 2004".into()}]);
    let mut renderer = Renderer::default();
    let mut metrics = RendererTextMetrics::default();
    let size = Size {
        width: 720.,
        height: 650.,
    };
    let frame = ui.frame(size, 1., &mut metrics);
    renderer
        .render(&frame.scene, 1.)?
        .save_png(format!("{directory}/list.png"))?;
    ui.update(NoticesInput::Activate(NoticesTarget::Component(0)));
    let frame = ui.frame(size, 1., &mut metrics);
    renderer
        .render(&frame.scene, 1.)?
        .save_png(format!("{directory}/license.png"))?;
    Ok(())
}
