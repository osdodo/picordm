use anyhow::Result;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, CurrentScreen};
use crate::connection::FormField;
use crate::ui;

pub async fn handle_key_event(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    key: KeyEvent,
) -> Result<bool> {
    if app.current_screen == CurrentScreen::JsonEditor {
        handle_json_editor(app, key);
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
        CurrentScreen::JsonEditor => {
            // Already handled above
        }
    }

    Ok(false)
}

fn handle_json_editor(app: &mut App<'_>, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.current_screen = CurrentScreen::KeyContent;
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.current_value = app.json_editor.lines().join("\n");
            app.current_screen = CurrentScreen::KeyContent;
        }
        _ => {
            app.json_editor.input(key);
        }
    }
}

async fn handle_global_shortcuts(app: &mut App<'_>, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') => true,
        KeyCode::Char('b') => {
            if !(app.current_screen == CurrentScreen::Dashboard && app.is_searching_keys) {
                if app.current_screen == CurrentScreen::Dashboard {
                    app.disconnect_and_return_to_list().await;
                } else if app.current_screen == CurrentScreen::KeyContent {
                    app.current_screen = CurrentScreen::Dashboard;
                }
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
            handle_form_char_input(app, c);
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
        KeyCode::Delete | KeyCode::Backspace => {
            if let Err(e) = app.delete_selected_connection() {
                app.error_message = Some(format!("Failed to delete connection: {}", e));
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.next_connection();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.previous_connection();
        }
        KeyCode::Enter => {
            app.start_connection();
        }
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
        KeyCode::Down | KeyCode::Char('j') => {
            app.next_db();
        }
        KeyCode::Up | KeyCode::Char('k') => {
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

async fn handle_key_search(
    terminal: &mut DefaultTerminal,
    app: &mut App<'_>,
    key: KeyEvent,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.is_searching_keys = false;
        }
        KeyCode::Char(c) => {
            app.key_search_filter.push(c);
            if !app.get_filtered_keys().is_empty() {
                app.key_list_state.select(Some(0));
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
                            app.fetch_value().await?;
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
        KeyCode::Char(' ') => {
            if app.current_screen == CurrentScreen::Dashboard {
                app.toggle_key_selection();
            }
        }
        KeyCode::Char('a') => {
            if app.current_screen == CurrentScreen::Dashboard {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.clear_key_selection();
                } else {
                    app.select_all_keys();
                }
            }
        }
        KeyCode::Char('x') => {
            if app.current_screen == CurrentScreen::Dashboard && !app.selected_keys.is_empty() {
                app.open_delete_confirmation();
            }
        }
        KeyCode::Char('d') => {
            if app.current_screen == CurrentScreen::Dashboard {
                app.toggle_db_selector();
            }
        }
        KeyCode::Char('/') => {
            if app.current_screen == CurrentScreen::Dashboard {
                app.is_searching_keys = true;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.current_screen == CurrentScreen::Dashboard {
                app.next_key();
            } else if app.current_screen == CurrentScreen::KeyContent {
                app.scroll_down();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.current_screen == CurrentScreen::Dashboard {
                app.previous_key();
            } else if app.current_screen == CurrentScreen::KeyContent {
                app.scroll_up();
            }
        }
        KeyCode::Enter => {
            if app.current_screen == CurrentScreen::Dashboard {
                app.is_loading_value = true;
                terminal.draw(|f| ui::draw(f, app))?;
                app.fetch_value().await?;
            }
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.is_loading_server_info = true;
            terminal.draw(|f| ui::draw(f, app))?;
            let _ = app.load_server_info().await;
        }
        KeyCode::Char('e') => {
            if app.current_screen == CurrentScreen::KeyContent && app.is_json_content {
                app.current_screen = CurrentScreen::JsonEditor;
            }
        }
        _ => {}
    }
    Ok(())
}
