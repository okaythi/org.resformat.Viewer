use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box, EventControllerScroll, FileDialog, FileFilter,
    GestureDrag, Orientation, Picture, PopoverMenuBar, ScrolledWindow,
};
use gtk4::gio::{self, ActionEntry, ApplicationFlags, Menu, MenuItem};
use gtk4::glib;
use res_core::decode::decode_res_to_rgb;
use std::cell::RefCell;
use std::rc::Rc;

const APP_ID: &str = "org.resformat.Viewer";

// State container to hold the active image dimensions and zoom multiplier
struct ImageState {
    base_width: i32,
    base_height: i32,
    zoom: f64,
    // Target position for the panning logic, updated on frame timer to remove flickering
    panning_target_x: f64,
    panning_target_y: f64,
    panning_timer: Option<glib::SourceId>,
}

fn main() -> glib::ExitCode {
    // 1. HANDLES_OPEN intercepts double-clicks from the OS file manager
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_activate(|app| {
        create_window(app, None);
    });

    app.connect_open(|app, files, _hint| {
        for file in files {
            create_window(app, Some(file.clone()));
        }
    });

    app.run()
}

fn create_window(app: &Application, initial_file: Option<gio::File>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("RES Viewer")
        .default_width(900)
        .default_height(700)
        .build();

    let vbox = Box::new(Orientation::Vertical, 0);

    // ==========================================
    // 1. MENU BAR BUILDER
    // ==========================================
    let file_menu = Menu::new();
    file_menu.append_item(&MenuItem::new(Some("Open..."), Some("win.open")));
    file_menu.append_item(&MenuItem::new(Some("Open in new window"), Some("win.open_new")));

    let menubar = Menu::new();
    menubar.append_submenu(Some("File"), &file_menu);
    
    let menu_widget = PopoverMenuBar::from_model(Some(&menubar));
    vbox.append(&menu_widget);

    // ==========================================
    // 2. ZOOM & PAN ENGINE
    // ==========================================
    let scrolled_window = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .kinetic_scrolling(true)
        .build();

    let picture = Picture::new();
    picture.set_can_shrink(false);

    let state = Rc::new(RefCell::new(ImageState {
        base_width: 0,
        base_height: 0,
        zoom: 1.0,
        panning_target_x: 0.0,
        panning_target_y: 0.0,
        panning_timer: None,
    }));

    // --- ZOOM LOGIC (Mouse Wheel) ---
    let scroll_ctrl = EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
    let pic_clone_zoom = picture.clone();
    let state_clone_zoom = state.clone();

    scroll_ctrl.connect_scroll(move |_, _dx, dy| {
        let mut st = state_clone_zoom.borrow_mut();
        if st.base_width == 0 {
            return glib::Propagation::Proceed;
        }

        if dy < 0.0 {
            st.zoom *= 1.15;
        } else if dy > 0.0 {
            st.zoom /= 1.15;
        }

        st.zoom = st.zoom.clamp(0.05, 20.0);

        let new_w = (st.base_width as f64 * st.zoom) as i32;
        let new_h = (st.base_height as f64 * st.zoom) as i32;
        pic_clone_zoom.set_size_request(new_w, new_h);

        glib::Propagation::Stop
    });

    // --- PAN LOGIC (Click & Drag with Frame Throttling) ---
    let drag_ctrl = GestureDrag::new();
    let drag_start_x = Rc::new(RefCell::new(0.0));
    let drag_start_y = Rc::new(RefCell::new(0.0));
    
    let sw_clone_begin = scrolled_window.clone();
    let start_x_clone = drag_start_x.clone();
    let start_y_clone = drag_start_y.clone();

    drag_ctrl.connect_drag_begin(move |_, _x, _y| {
        *start_x_clone.borrow_mut() = sw_clone_begin.hadjustment().value();
        *start_y_clone.borrow_mut() = sw_clone_begin.vadjustment().value();
    });

    let sw_weak_update = scrolled_window.downgrade();
    let state_clone_panning = state.clone();

    drag_ctrl.connect_drag_update(move |_, dx, dy| {
        let mut st = state_clone_panning.borrow_mut();
        
        st.panning_target_x = *drag_start_x.borrow() - dx;
        st.panning_target_y = *drag_start_y.borrow() - dy;

        if st.panning_timer.is_some() {
            return;
        }

        let state_weak_timer = Rc::downgrade(&state_clone_panning);
        let sw_weak_timer = sw_weak_update.clone();

        st.panning_timer = Some(glib::timeout_add_local(
            std::time::Duration::from_millis(16),
            move || {
                if let (Some(st_strong), Some(sw_strong)) = (state_weak_timer.upgrade(), sw_weak_timer.upgrade()) {
                    let mut st = st_strong.borrow_mut();
                    
                    let hadj_timer = sw_strong.hadjustment();
                    let vadj_timer = sw_strong.vadjustment();

                    hadj_timer.set_value(st.panning_target_x);
                    vadj_timer.set_value(st.panning_target_y);

                    st.panning_timer = None;
                    
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Break
                }
            },
        ));
    });

    picture.add_controller(scroll_ctrl);
    picture.add_controller(drag_ctrl);

    scrolled_window.set_child(Some(&picture));
    vbox.append(&scrolled_window);
    window.set_child(Some(&vbox));

    // ==========================================
    // 3. ACTION WIRING (Menu Bar Clicks)
    // ==========================================
    let pic_weak = picture.downgrade();
    let state_weak = Rc::downgrade(&state);
    let win_weak = window.downgrade();

    let action_open = gio::SimpleAction::new("open", None);
    action_open.connect_activate(move |_, _| {
        if let (Some(win), Some(pic), Some(st)) = (win_weak.upgrade(), pic_weak.upgrade(), state_weak.upgrade()) {
            open_file_dialog(&win, &pic, &st);
        }
    });
    window.add_action(&action_open);

    let app_weak = app.downgrade();
    let action_open_new = gio::SimpleAction::new("open_new", None);
    action_open_new.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            create_window(&app, None);
        }
    });
    window.add_action(&action_open_new);

    // ==========================================
    // 4. INITIALIZATION
    // ==========================================
    window.present();

    if let Some(file) = initial_file {
        load_res_file(&picture, &scrolled_window, &state, file);
    } else {
        open_file_dialog(&window, &picture, &state);
    }
}

// ==========================================
// CORE DECODING & OS DIALOGS
// ==========================================
fn open_file_dialog(window: &ApplicationWindow, picture: &Picture, state: &Rc<RefCell<ImageState>>) {
    let filter = FileFilter::new();
    filter.add_pattern("*.res");
    filter.set_name(Some("RES Images"));

    let filters = gio::ListStore::new::<FileFilter>();
    filters.append(&filter);

    let dialog = FileDialog::builder()
        .title("Open .res Image")
        .default_filter(&filter)
        .filters(&filters)
        .build();

    let pic_clone = picture.clone();
    let state_clone = state.clone();
    
    dialog.open(Some(window), gio::Cancellable::NONE, move |result| {
        if let Ok(file) = result {
            let scrolled_window = pic_clone.parent().unwrap().downcast::<ScrolledWindow>().unwrap();
            load_res_file(&pic_clone, &scrolled_window, &state_clone, file);
        }
    });
}

fn load_res_file(picture: &Picture, scrolled_window: &ScrolledWindow, state: &Rc<RefCell<ImageState>>, file: gio::File) {
    if let Ok((bytes, _)) = file.load_contents(gio::Cancellable::NONE) {
        if let Ok((w, h, rgb)) = decode_res_to_rgb(&bytes) {
            let glib_bytes = glib::Bytes::from_owned(rgb);
            let texture = gtk::gdk::MemoryTexture::new(
                w as i32,
                h as i32,
                gtk::gdk::MemoryFormat::R8g8b8,
                &glib_bytes,
                (w * 3) as usize,
            );
            picture.set_paintable(Some(&texture));

            let mut st = state.borrow_mut();
            st.base_width = w as i32;
            st.base_height = h as i32;
            st.panning_target_x = 0.0;
            st.panning_target_y = 0.0;
            
            // Fix: Oxidized method call for removing the timer
            if let Some(timer_id) = st.panning_timer.take() {
                timer_id.remove(); 
            }

            scrolled_window.queue_allocate(); 

            let window_w = scrolled_window.width() as f64;
            let window_h = scrolled_window.height() as f64;

            if window_w > 0.0 && window_h > 0.0 && w > 0 && h > 0 {
                let fit_ratio = (window_w / w as f64).min(window_h / h as f64);
                st.zoom = fit_ratio.clamp(0.05, 20.0);
            } else {
                st.zoom = 1.0; 
            }

            let final_w = (st.base_width as f64 * st.zoom) as i32;
            let final_h = (st.base_height as f64 * st.zoom) as i32;
            picture.set_size_request(final_w, final_h);
        
        } else {
            eprintln!("Failed to decode .res file.");
        }
    }
}