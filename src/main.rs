use gtk::glib;
use gtk::glib::Propagation;
use gtk::{prelude::*, CssProvider};
use gtk::{Application, ApplicationWindow, Box, Orientation, Label, Button};
use gtk4_layer_shell::{Layer, LayerShell, KeyboardMode};

fn main() {
    let app = Application::builder()
        .application_id("ru.pinger.daemon")
        .build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("pinger")
        .default_width(400)
        .default_height(200)
        .build();
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::OnDemand);

    let provider = CssProvider::new();
    provider.load_from_path("src/style.css");
    gtk::style_context_add_provider_for_display(
        &gtk::prelude::WidgetExt::display(&window),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION
    );
    
    let ask_box = Box::new(Orientation::Vertical, 10);
    let ask_label = Label::new(Some("Писать pinger"));
    ask_box.set_margin_top(10);
    ask_box.append(&ask_label);

    let buttons_box = Box::new(Orientation::Horizontal, 30);
    buttons_box.set_halign(gtk::Align::Center);

    let ans1_button = Button::with_label("Делаю");
    let ans2_button = Button::with_label("Закрыть");
    buttons_box.append(&ans1_button);
    buttons_box.append(&ans2_button);
    ask_box.append(&buttons_box);

    window.set_child(Some(&ask_box));

    let window_clone = window.clone();
    ans2_button.connect_clicked(move |_| {
        window_clone.set_visible(false);
    });

    let controller = gtk::EventControllerKey::new();
    let ans1_clone = ans1_button.clone();
    let ans2_clone = ans2_button.clone();
    controller.connect_key_pressed(move |_, _, keycode, _| {
        match keycode {
            41 => {
                ans1_clone.emit_clicked();
            }
            44 => {
                ans2_clone.emit_clicked();
            }
            _ => {}
        }
        Propagation::Proceed
    });
    window.add_controller(controller);

    let window_clone = window.clone();
    glib::spawn_future_local(async move {
    });

    window.present();
}
