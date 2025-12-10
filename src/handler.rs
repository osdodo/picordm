use anyhow::Result;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

use crate::app::{App, CurrentScreen};
use crate::connection::FormField;
use crate::ui;

pub async fn handle_key_event(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    key: KeyEvent,
) -> Result<bool> {
    if app.current_screen == CurrentScreen::JsonEditor {
        handle_json_editor(terminal, app, key).await?;
        return Ok(false);
    }

    if handle_global_shortcuts(app, key).await {
        return Ok(true);
    }

    match app.current_screen {
        CurrentScreen::NewConnectionForm => {
            handle_connection_form(app, key)?;
        }
        CurrentScreen::ConnectionList => {
            handle_connection_list(terminal, app, key).await?;
        }
        CurrentScreen::Dashboard | CurrentScreen::KeyContent => {
            handle_dashboard_and_key_content(terminal, app, key).await?;
        }
        CurrentScreen::CommandMode => {
            handle_command_mode(terminal, app, key).await?;
        }
        CurrentScreen::JsonEditor => {
            // Already handled above
        }
    }

    Ok(false)
}

async fn handle_json_editor(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    key: KeyEvent,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.current_screen = CurrentScreen::KeyContent;
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_value = app.json_editor.lines().join("\n");
            app.cached_highlighted_json = None; // Clear cache after editing
            terminal.draw(|f| ui::draw(f, app))?;
            app.save_current_value().await?;
            app.current_screen = CurrentScreen::KeyContent;
        }
        _ => {
            app.json_editor.input(key);
        }
    }
    Ok(())
}

async fn handle_global_shortcuts(app: &mut App<'_>, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
        KeyCode::Char('b') => {
            if app.is_searching_keys
                || app.current_screen == CurrentScreen::CommandMode
                || app.current_screen == CurrentScreen::NewConnectionForm
            {
                return false;
            }

            if app.current_screen == CurrentScreen::Dashboard {
                app.disconnect_and_return_to_list().await;
            } else if app.current_screen == CurrentScreen::KeyContent {
                app.current_screen = CurrentScreen::Dashboard;
            }
            false
        }
        _ => false,
    }
}

fn handle_connection_form(app: &mut App<'_>, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.current_screen = CurrentScreen::ConnectionList;
            app.error_message = None;
        }
        KeyCode::Tab => {
            app.next_form_field();
        }
        KeyCode::BackTab => {
            app.previous_form_field();
        }
        KeyCode::Enter => {
            handle_form_enter(app)?;
        }
        KeyCode::Char(c) => {
            // Ignore character input with Ctrl, Alt, or Super (Cmd) modifier keys
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT)
                && !key.modifiers.contains(KeyModifiers::SUPER)
            {
                handle_form_char_input(app, c);
            }
        }
        KeyCode::Backspace => {
            handle_form_backspace(app);
        }
        _ => {}
    }
    Ok(())
}

fn handle_form_enter(app: &mut App<'_>) -> Result<()> {
    match app.connection_form.editing_field {
        FormField::UseTls => {
            app.connection_form.use_tls = !app.connection_form.use_tls;
        }
        FormField::AllowInsecureTls => {
            app.connection_form.allow_insecure_tls = !app.connection_form.allow_insecure_tls;
        }
        FormField::Submit => {
            app.save_connection_form()?;
        }
        _ => {
            app.next_form_field();
        }
    }
    Ok(())
}

fn handle_form_char_input(app: &mut App<'_>, c: char) {
    match app.connection_form.editing_field {
        FormField::Name => app.connection_form.name.push(c),
        FormField::Host => app.connection_form.host.push(c),
        FormField::Port => app.connection_form.port.push(c),
        FormField::Username => {
            if let Some(ref mut u) = app.connection_form.username {
                u.push(c);
            } else {
                app.connection_form.username = Some(c.to_string());
            }
        }
        FormField::Password => {
            if let Some(ref mut p) = app.connection_form.password {
                p.push(c);
            } else {
                app.connection_form.password = Some(c.to_string());
            }
        }
        FormField::Sni => app.connection_form.sni.push(c),
        FormField::DbAliases => app.connection_form.db_aliases.push(c),
        FormField::UseTls | FormField::AllowInsecureTls => {
            if c == ' ' {
                if app.connection_form.editing_field == FormField::UseTls {
                    app.connection_form.use_tls = !app.connection_form.use_tls;
                } else {
                    app.connection_form.allow_insecure_tls =
                        !app.connection_form.allow_insecure_tls;
                }
            }
        }
        _ => {}
    }
}

fn handle_form_backspace(app: &mut App<'_>) {
    match app.connection_form.editing_field {
        FormField::Name => {
            app.connection_form.name.pop();
        }
        FormField::Host => {
            app.connection_form.host.pop();
        }
        FormField::Port => {
            app.connection_form.port.pop();
        }
        FormField::Username => {
            if let Some(ref mut u) = app.connection_form.username {
                u.pop();
            }
        }
        FormField::Password => {
            if let Some(ref mut p) = app.connection_form.password {
                p.pop();
            }
        }
        FormField::Sni => {
            app.connection_form.sni.pop();
        }
        FormField::DbAliases => {
            app.connection_form.db_aliases.pop();
        }
        _ => {}
    }
}

async fn handle_connection_list(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    key: KeyEvent,
) -> Result<()> {
    // Only respond to keys without modifier keys
    if key.modifiers.contains(KeyModifiers::CONTROL)
        || key.modifiers.contains(KeyModifiers::ALT)
        || key.modifiers.contains(KeyModifiers::SUPER)
    {
        return Ok(());
    }

    match key.code {
        KeyCode::Char('n') => {
            app.current_screen = CurrentScreen::NewConnectionForm;
            app.error_message = None;
        }
        KeyCode::Char('e') => {
            app.load_connection_for_edit();
        }
        KeyCode::Char('i') => {
            terminal.draw(|f| ui::draw(f, app))?;
            app.quick_import_from_clipboard().await?;
        }
        KeyCode::Down => {
            app.next_connection();
        }
        KeyCode::Up => {
            app.previous_connection();
        }
        KeyCode::Delete | KeyCode::Backspace => {
            if let Err(e) = app.delete_selected_connection() {
                app.error_message = Some(format!("Failed to delete connection: {}", e));
            }
        }
        KeyCode::Enter => {
            app.start_connection();
        }
        // Ignore all other character inputs to prevent unexpected actions during paste.
        KeyCode::Char(_) => {}
        _ => {}
    }
    Ok(())
}

async fn handle_dashboard_and_key_content(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    key: KeyEvent,
) -> Result<()> {
    if app.current_screen == CurrentScreen::Dashboard && app.is_db_selector_open {
        handle_db_selector(terminal, app, key).await?;
    } else if app.current_screen == CurrentScreen::Dashboard && app.is_searching_keys {
        handle_key_search(terminal, app, key).await?;
    } else {
        handle_dashboard_normal(terminal, app, key).await?;
    }
    Ok(())
}

async fn handle_db_selector(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    key: KeyEvent,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.is_db_selector_open = false;
        }
        KeyCode::Down => {
            app.next_db();
        }
        KeyCode::Up => {
            app.previous_db();
        }
        KeyCode::Enter => {
            if let Some(selected_idx) = app.db_selector_state.selected() {
                if let Some(db) = app.db_list.get(selected_idx) {
                    let db_index = db.index;
                    app.is_loading_keys = true;
                    terminal.draw(|f| ui::draw(f, app))?;
                    app.switch_db(db_index).await?;
                }
            }
            app.is_db_selector_open = false;
        }
        _ => {}
    }
    Ok(())
}

async fn handle_command_mode(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    key: KeyEvent,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.toggle_command_mode();
        }
        KeyCode::Down => {
            app.scroll_down();
        }
        KeyCode::Up => {
            app.scroll_up();
        }
        KeyCode::Char(c) => {
            // Ignore character input with Ctrl, Alt, or Super (Cmd) modifier keys
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT)
                && !key.modifiers.contains(KeyModifiers::SUPER)
            {
                app.command_input.push(c);
            }
        }
        KeyCode::Backspace => {
            app.command_input.pop();

        }
        KeyCode::Enter => {
            terminal.draw(|f| ui::draw(f, app))?;
            app.execute_command().await?;
            // Stay in command mode, allowing continuous command input.
        }
        _ => {}
    }
    Ok(())
}

async fn handle_key_search(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    key: KeyEvent,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.is_searching_keys = false;
        }
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Support Ctrl+a (select all) in search mode
            let filtered_keys = app.get_filtered_keys();
            let all_selected = !filtered_keys.is_empty() 
                && filtered_keys.iter().all(|k| app.selected_keys.contains(k));
            
            if all_selected {
                app.clear_key_selection();
            } else {
                app.select_all_keys();
            }
        }
        KeyCode::Char(' ') => {
            // Support space key (toggle selection) in search mode
            app.toggle_key_selection();
        }
        KeyCode::Char(c) => {
            // Ignore character input with Ctrl, Alt, or Super (Cmd) modifier keys
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT)
                && !key.modifiers.contains(KeyModifiers::SUPER)
            {
                app.key_search_filter.push(c);
                if !app.get_filtered_keys().is_empty() {
                    app.key_list_state.select(Some(0));
                }
            }
        }
        KeyCode::Backspace => {
            app.key_search_filter.pop();
            if !app.get_filtered_keys().is_empty() {
                app.key_list_state.select(Some(0));
            }
        }
        KeyCode::Enter => {
            app.is_searching_keys = false;
            if !app.get_filtered_keys().is_empty() {
                if let Some(selected_idx) = app.key_list_state.selected() {
                    let filtered_keys = app.get_filtered_keys();
                    if let Some(selected_key) = filtered_keys.get(selected_idx) {
                        if let Some(original_idx) = app.keys.iter().position(|k| k == selected_key)
                        {
                            app.key_list_state.select(Some(original_idx));
                            app.is_loading_value = true;
                            terminal.draw(|f| ui::draw(f, app))?;
                            app.fetch_value(true).await?;
                        }
                    }
                }
            }
        }
        KeyCode::Down | KeyCode::Up => {
            let filtered_keys = app.get_filtered_keys();
            if !filtered_keys.is_empty() {
                let i = match app.key_list_state.selected() {
                    Some(i) => {
                        if key.code == KeyCode::Down {
                            if i >= filtered_keys.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        } else {
                            if i == 0 {
                                filtered_keys.len() - 1
                            } else {
                                i - 1
                            }
                        }
                    }
                    None => 0,
                };
                app.key_list_state.select(Some(i));
            }
        }
        _ => {}
    }
    Ok(())
}

async fn handle_dashboard_normal(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    key: KeyEvent,
) -> Result<()> {
    // Handle delete confirmation dialog
    if app.is_delete_confirmation_open {
        // Ignore input with modifier keys
        if key.modifiers.contains(KeyModifiers::ALT) || key.modifiers.contains(KeyModifiers::SUPER)
        {
            return Ok(());
        }

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                terminal.draw(|f| ui::draw(f, app))?;
                app.delete_selected_keys().await?;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.close_delete_confirmation();
            }
            _ => {}
        }
        return Ok(());
    }

    match key.code {
        // Special handling for the Ctrl key combination (must be before wildcards)
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_screen == CurrentScreen::Dashboard {
                // Toggle: if all keys are selected, clear selection; otherwise select all
                let filtered_keys = app.get_filtered_keys();
                let all_selected = !filtered_keys.is_empty() 
                    && filtered_keys.iter().all(|k| app.selected_keys.contains(k));
                
                if all_selected {
                    app.clear_key_selection();
                } else {
                    app.select_all_keys();
                }
            }
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_screen == CurrentScreen::Dashboard {
                app.toggle_db_selector();
            }
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.is_loading_server_info = true;
            terminal.draw(|f| ui::draw(f, app))?;
            let _ = app.load_server_info().await;
        }
        // Ignore other inputs with modifier keys
        KeyCode::Char(_)
            if key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::ALT)
                || key.modifiers.contains(KeyModifiers::SUPER) => {}
        // Handling regular buttons
        KeyCode::Char(' ') => {
            if app.current_screen == CurrentScreen::Dashboard {
                app.toggle_key_selection();
            }
        }

        KeyCode::Delete | KeyCode::Backspace => {
            if app.current_screen == CurrentScreen::Dashboard && !app.selected_keys.is_empty() {
                app.open_delete_confirmation();
            }
        }
        KeyCode::Char('/') => {
            if app.current_screen == CurrentScreen::Dashboard {
                app.is_searching_keys = true;
            }
        }
        KeyCode::Char('>') => {
            if app.current_screen == CurrentScreen::Dashboard || app.current_screen == CurrentScreen::KeyContent {
                app.toggle_command_mode();
            }
        }
        KeyCode::Down => {
            if app.current_screen == CurrentScreen::Dashboard {
                app.next_key();
            } else if app.current_screen == CurrentScreen::KeyContent {
                app.scroll_down();
            }
        }
        KeyCode::Up => {
            if app.current_screen == CurrentScreen::Dashboard {
                app.previous_key();
            } else if app.current_screen == CurrentScreen::KeyContent {
                app.scroll_up();
            }
        }
        KeyCode::Char('e') => {
            if (app.current_screen == CurrentScreen::KeyContent
                || app.current_screen == CurrentScreen::Dashboard)
                && app.is_json_content
                && !app.current_value.is_empty()
            {
                app.current_screen = CurrentScreen::JsonEditor;
            }
        }
        KeyCode::Enter => {
            if app.current_screen == CurrentScreen::Dashboard {
                app.is_loading_value = true;
                terminal.draw(|f| ui::draw(f, app))?;
                app.fetch_value(true).await?;
            }
        }
        // Ignore all other character inputs to prevent unexpected actions during paste.
        KeyCode::Char(_) => {}
        _ => {}
    }
    Ok(())
}

pub async fn handle_mouse_event(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    mouse: MouseEvent,
) -> Result<()> {
    // Skip mouse events in form and editor screens
    if app.current_screen == CurrentScreen::NewConnectionForm
        || app.current_screen == CurrentScreen::JsonEditor
    {
        return Ok(());
    }

    match mouse.kind {
        MouseEventKind::ScrollDown => {
            handle_mouse_scroll_down(app);
        }
        MouseEventKind::ScrollUp => {
            handle_mouse_scroll_up(app);
        }
        MouseEventKind::Down(_) => {
            handle_mouse_click(terminal, app, mouse.column, mouse.row).await?;
        }
        _ => {}
    }

    Ok(())
}

fn handle_mouse_scroll_down(app: &mut App<'_>) {
    match app.current_screen {
        CurrentScreen::ConnectionList => {
            app.next_connection();
        }
        CurrentScreen::Dashboard => {
            if app.is_db_selector_open {
                app.next_db();
            } else {
                app.next_key();
            }
        }
        CurrentScreen::CommandMode => {
            app.scroll_down();
        }
        CurrentScreen::KeyContent => {
            app.scroll_down();
        }
        _ => {}
    }
}

fn handle_mouse_scroll_up(app: &mut App<'_>) {
    match app.current_screen {
        CurrentScreen::ConnectionList => {
            app.previous_connection();
        }
        CurrentScreen::Dashboard => {
            if app.is_db_selector_open {
                app.previous_db();
            } else {
                app.previous_key();
            }
        }
        CurrentScreen::CommandMode => {
            app.scroll_up();
        }
        CurrentScreen::KeyContent => {
            app.scroll_up();
        }
        _ => {}
    }
}

async fn handle_mouse_click(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    col: u16,
    row: u16,
) -> Result<()> {
    let size = terminal.size()?;

    // Calculate layout areas (matching ui.rs layout)
    let header_height = 3;
    let footer_height = 3;
    let main_start = header_height;
    let main_end = size.height.saturating_sub(footer_height);

    // Check if click is in main area
    if row < main_start || row >= main_end {
        return Ok(());
    }

    let main_row = row - main_start;

    match app.current_screen {
        CurrentScreen::ConnectionList => {
            handle_connection_list_click(app, main_row);
        }
        CurrentScreen::Dashboard | CurrentScreen::KeyContent => {
            handle_dashboard_click(terminal, app, col, main_row, size.width).await?;
        }
        _ => {}
    }

    Ok(())
}

fn handle_connection_list_click(app: &mut App<'_>, row: u16) {
    // Sidebar is 30% of width, click in that area
    // List starts at row 1 (after border)
    if row > 0 {
        let list_row = (row - 1) as usize;
        let connections_count = app.connection_list.connections().len();
        if list_row < connections_count {
            app.connection_list.state().select(Some(list_row));
        }
    }
}

async fn handle_dashboard_click(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    col: u16,
    row: u16,
    width: u16,
) -> Result<()> {
    let sidebar_width = (width * 30) / 100;

    // Click in sidebar (keys list)
    if col < sidebar_width {
        handle_keys_list_click(terminal, app, row).await?;
    }

    Ok(())
}

async fn handle_keys_list_click(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    row: u16,
) -> Result<()> {
    // Search box is 3 lines, keys list starts after that
    let search_box_height = 3;

    if row > search_box_height {
        // Approximate keys list area
        let list_row = (row - search_box_height - 1) as usize; // -1 for border
        let filtered_keys = app.get_filtered_keys();

        if list_row < filtered_keys.len() {
            // Get the selected key from filtered list
            let selected_key = &filtered_keys[list_row];
            
            // Find the original index in the full keys list
            if let Some(original_idx) = app.keys.iter().position(|k| k == selected_key) {
                // Update selection to original index
                app.key_list_state.select(Some(original_idx));

                // Exit command mode if active
                if app.current_screen == CurrentScreen::CommandMode {
                    app.toggle_command_mode();
                }

                // Load value and switch to KeyContent screen
                app.is_loading_value = true;
                terminal.draw(|f| ui::draw(f, app))?;
                app.fetch_value(true).await?;
            }
        }
    }

    Ok(())
}
