use std::fs;
use rand::Rng;
use chrono::{Local, Timelike};

use gtk::glib;
use gtk::glib::Propagation;
use gtk::{prelude::*, CssProvider};
use gtk::{Application, ApplicationWindow, Box, Orientation, Label, Button};
use gtk4_layer_shell::{Layer, LayerShell, KeyboardMode};

fn get_now() -> i32 {
    let now = Local::now();
    return (now.hour() * 60 + now.minute()) as i32;
}

fn main() {
    let app = Application::builder()
        .application_id("ru.pinger.daemon")
        .build();
    app.connect_activate(build_ui);
    app.run();
    //println!("{:?}", read_config("test.txt"));
}

#[derive(Clone)]
struct AppWidgets {
    window: ApplicationWindow,
    label: Label,
    ans1: Button,
    ans2: Button,
}

#[derive(Debug)]
struct Task {
    start: Option<i32>,
    duration: Option<i32>,
    title: String,
    code: String,
}

fn parse_time(t: &str) -> i32 {
    let parts: Vec<&str> = t.split(':').collect();
    match parts.len() {
        1 => parts[0].parse::<i32>().expect("Expected i32 in time!"),
        2 => {
            let h = parts[0].parse::<i32>().expect("Expected i32 in time!");
            let m = parts[1].parse::<i32>().expect("Expected i32 in time!");
            h * 60 + m
        }
        _ => panic!("You can use only mm or hh:mm")
    }
}

fn read_config(path: &str) -> Vec<Task> {
    let mut rng = rand::thread_rng();
    fs::read_to_string(path)
        .expect("Cannot read config!")
        .lines().map(String::from)
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            let mut start = None;
            let mut duration = None;
            let mut title_parts = Vec::new();
            let mut code_opt = None;
            
            for p in s.split(' ') {
                if p.is_empty() {continue;}
                
                if p.starts_with('$') {
                    start = Some(parse_time(&p[1..]));
                } else if p.starts_with('~') {
                    duration = Some(parse_time(&p[1..]));
                } else if p.starts_with('@') {
                    code_opt = Some(p[1..].to_string());
                } else {
                    title_parts.push(p);
                }
            }

            //let code = code_opt.unwrap_or_else(|| rng.gen_range(1000..99999).to_string());
            let code = code_opt.unwrap_or(start.unwrap().to_string());

            Task{start, duration, title: title_parts.join(" "), code}
        }).collect()
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
    window.set_visible(false);

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

    let widgets = AppWidgets{
        window: window.clone(),
        label: ask_label.clone(),
        ans1: ans1_button.clone(),
        ans2: ans2_button.clone()
    };
    run_pinger_loop(widgets);

    window.present();
}

fn run_pinger_loop(widgets: AppWidgets) {
    glib::spawn_future_local(async move {
        loop {
            glib::timeout_future(std::time::Duration::from_secs(5)).await;
            let tasks = read_config("test.txt");
            for task in tasks {
                if task.start.unwrap_or(0) <= get_now() {
                    widgets.label.set_text(&task.title);
                    widgets.window.set_visible(true);
                    break;
                }
            }
        }
    });
}
