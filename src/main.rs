#![windows_subsystem = "windows"]
slint::include_modules!(); // loads the generated code

fn main() -> Result<(), slint::PlatformError> {
    let main_window = MainWindow::new()?;
    main_window.run()
    
}

