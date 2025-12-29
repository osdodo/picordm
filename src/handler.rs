use anyhow::Result;
use edtui::EditorState;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{App, CurrentScreen};
use crate::connection::FormField;
use crate::ui;

// Check for modifier keys (ignore Shift)
fn has_modifiers(key: &KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        || key.modifiers.contains(KeyModifiers::ALT)
        || key.modifiers.contains(KeyModifiers::SUPER)
}

// Pass key events to the edtui editor
fn pass_to_editor(app: &mut App, key: KeyEvent) {
    app.editor_event_handler
        .on_key_event(key, &mut app.editor_state);
}

pub async fn handle_key_event(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    key: KeyEvent,
) -> Result<bool> {
    // On Windows, crossterm may fire both Press and Release events
    // We only want to handle Press events to avoid duplicate input
    if key.kind != KeyEventKind::Press {
        return Ok(false);
    }
    // Handle progress dialog first - block most input during operations
    if let Some(ref dialog) = app.progress_dialog {
        if dialog.is_complete {
            // Allow Esc to close completed dialog
            if key.code == KeyCode::Esc {
                app.hide_progress_dialog();
            }
        }
        // Block all other input during progress operations
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
        CurrentScreen::FileSelector => {
            handle_file_selector(terminal, app, key).await?;
        }
        CurrentScreen::ConnectionSwitcher => {
            handle_connection_switcher(terminal, app, key).await?;
        }
    }

    Ok(false)
}

async fn handle_global_shortcuts(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.is_searching_keys
                || app.current_screen == CurrentScreen::CommandMode
                || app.current_screen == CurrentScreen::NewConnectionForm
                || app.current_screen == CurrentScreen::ConnectionSwitcher
            {
                return false;
            }

            if app.current_screen == CurrentScreen::Dashboard {
                app.disconnect_and_return_to_list().await;
            }
            false
        }
        KeyCode::Char('b') => {
            if app.is_searching_keys
                || app.current_screen == CurrentScreen::CommandMode
                || app.current_screen == CurrentScreen::NewConnectionForm
                || app.current_screen == CurrentScreen::ConnectionSwitcher
            {
                return false;
            }

            if app.current_screen == CurrentScreen::KeyContent {
                app.current_screen = CurrentScreen::Dashboard;
            }
            false
        }
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Quick connection switcher - only available in Dashboard and KeyContent
            if app.current_screen == CurrentScreen::Dashboard
                || app.current_screen == CurrentScreen::KeyContent
            {
                app.show_connection_switcher();
            }
            false
        }
        _ => false,
    }
}

fn handle_connection_form(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.current_screen = CurrentScreen::ConnectionList;
            app.error_message = None;
        }
        KeyCode::Tab => {
            app.connection_form.is_cluster = !app.connection_form.is_cluster;
        }
        KeyCode::Down => {
            app.next_form_field();
        }
        KeyCode::Up => {
            app.previous_form_field();
        }
        KeyCode::Enter => {
            handle_form_enter(app)?;
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.save_connection_form()?;
        }
        KeyCode::Char(c) => {
            if !has_modifiers(&key) {
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

fn handle_form_enter(app: &mut App) -> Result<()> {
    match app.connection_form.editing_field {
        FormField::UseTls => {
            app.connection_form.use_tls = !app.connection_form.use_tls;
        }
        FormField::AllowInsecureTls => {
            app.connection_form.allow_insecure_tls = !app.connection_form.allow_insecure_tls;
        }
        _ => {
            // For other fields, Enter moves to next field
            app.next_form_field();
        }
    }
    Ok(())
}

fn handle_form_char_input(app: &mut App, c: char) {
    use FormField::*;
    match app.connection_form.editing_field {
        Name => app.connection_form.name.push(c),
        Host => app.connection_form.host.push(c),
        Port => app.connection_form.port.push(c),
        Username => app
            .connection_form
            .username
            .get_or_insert_with(String::new)
            .push(c),
        Password => app
            .connection_form
            .password
            .get_or_insert_with(String::new)
            .push(c),
        Sni => app.connection_form.sni.push(c),
        ClusterNodes => app.connection_form.cluster_nodes.push(c),
        DbAliases => app.connection_form.db_aliases.push(c),
        UseTls if c == ' ' => app.connection_form.use_tls = !app.connection_form.use_tls,
        AllowInsecureTls if c == ' ' => {
            app.connection_form.allow_insecure_tls = !app.connection_form.allow_insecure_tls
        }
        _ => {}
    }
}

fn handle_form_backspace(app: &mut App) {
    use FormField::*;
    match app.connection_form.editing_field {
        Name => {
            app.connection_form.name.pop();
        }
        Host => {
            app.connection_form.host.pop();
        }
        Port => {
            app.connection_form.port.pop();
        }
        Username => {
            app.connection_form.username.as_mut().map(|s| s.pop());
        }
        Password => {
            app.connection_form.password.as_mut().map(|s| s.pop());
        }
        Sni => {
            app.connection_form.sni.pop();
        }
        ClusterNodes => {
            app.connection_form.cluster_nodes.pop();
        }
        DbAliases => {
            app.connection_form.db_aliases.pop();
        }
        UseTls | AllowInsecureTls => {}
    }
}

async fn handle_connection_list(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    key: KeyEvent,
) -> Result<()> {
    // Only respond to keys without modifier keys
    if has_modifiers(&key) {
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
    app: &mut App,
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
    app: &mut App,
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
            if let Some(selected_idx) = app.db_selector_state.selected()
                && let Some(db) = app.db_list.get(selected_idx)
            {
                let db_index = db.index;
                app.is_loading_keys = true;
                terminal.draw(|f| ui::draw(f, app))?;
                app.switch_db(db_index).await?;
                app.is_loading_keys = false;
            }
            app.is_db_selector_open = false;
        }
        _ => {}
    }
    Ok(())
}

async fn handle_command_mode(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    key: KeyEvent,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            if app.command_mode_focus_on_output {
                use edtui::EditorMode;
                match app.editor_state.mode {
                    EditorMode::Normal => app.toggle_command_mode(),
                    _ => pass_to_editor(app, key),
                }
            } else {
                app.toggle_command_mode();
            }
        }
        KeyCode::Tab => {
            if !app.command_output.is_empty() {
                app.command_mode_focus_on_output = !app.command_mode_focus_on_output;
            }
        }
        KeyCode::Down
        | KeyCode::Up
        | KeyCode::Left
        | KeyCode::Right
        | KeyCode::PageDown
        | KeyCode::PageUp
        | KeyCode::Home
        | KeyCode::End => {
            if !app.command_output.is_empty() && app.command_mode_focus_on_output {
                pass_to_editor(app, key);
            }
        }
        KeyCode::Char(c) => {
            if !app.command_mode_focus_on_output {
                if !has_modifiers(&key) {
                    app.command_input.push(c);
                }
            } else {
                pass_to_editor(app, key);
            }
        }
        KeyCode::Backspace => {
            if !app.command_mode_focus_on_output {
                app.command_input.pop();
            }
        }
        KeyCode::Enter => {
            if !app.command_mode_focus_on_output {
                terminal.draw(|f| ui::draw(f, app))?;
                app.execute_command().await?;
            }
        }
        _ => {}
    }
    Ok(())
}

async fn handle_key_search(
    terminal: &mut DefaultTerminal,
    app: &mut App,
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
            if !has_modifiers(&key) {
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
            if !app.get_filtered_keys().is_empty()
                && let Some(selected_idx) = app.key_list_state.selected()
            {
                let filtered_keys = app.get_filtered_keys();
                if let Some(selected_key) = filtered_keys.get(selected_idx)
                    && let Some(original_idx) = app.keys.iter().position(|k| k == selected_key)
                {
                    app.key_list_state.select(Some(original_idx));
                    app.is_loading_value = true;
                    terminal.draw(|f| ui::draw(f, app))?;
                    app.fetch_value(true).await?;
                    app.is_loading_value = false;
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
                        } else if i == 0 {
                            filtered_keys.len() - 1
                        } else {
                            i - 1
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
    app: &mut App,
    key: KeyEvent,
) -> Result<()> {
    // Handle delete confirmation dialog
    if app.is_delete_confirmation_open {
        if key.modifiers.contains(KeyModifiers::ALT) || key.modifiers.contains(KeyModifiers::SUPER)
        {
            return Ok(());
        }

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.show_progress_dialog("Deleting Keys".to_string(), "Preparing...".to_string());
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
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_screen == CurrentScreen::Dashboard {
                app.toggle_db_selector();
            }
        }
        KeyCode::F(5) => {
            app.is_loading_server_info = true;
            terminal.draw(|f| ui::draw(f, app))?;
            let _ = app.load_server_info().await;
            app.is_loading_server_info = false;
        }
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_screen == CurrentScreen::Dashboard
                || app.current_screen == CurrentScreen::KeyContent
            {
                // Show progress dialog and start export
                app.show_progress_dialog("Export Data".to_string(), "Preparing...".to_string());
                terminal.draw(|f| ui::draw(f, app))?;
                if let Err(e) = app.export_redis_data().await {
                    app.error_message = Some(format!("Export failed: {}", e));
                }
            }
        }
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_screen == CurrentScreen::Dashboard
                || app.current_screen == CurrentScreen::KeyContent
            {
                app.show_file_selector();
            }
        }
        // Ignore other inputs with modifier keys (except Ctrl which is handled above)
        KeyCode::Char(_)
            if key.modifiers.contains(KeyModifiers::ALT)
                || key.modifiers.contains(KeyModifiers::SUPER) => {}
        KeyCode::Char('/') => match app.current_screen {
            CurrentScreen::Dashboard => app.is_searching_keys = true,
            CurrentScreen::KeyContent => pass_to_editor(app, key),
            _ => {}
        },
        KeyCode::Char('>') => {
            if app.current_screen == CurrentScreen::Dashboard {
                app.toggle_command_mode();
            }
        }
        KeyCode::Char(' ') => match app.current_screen {
            CurrentScreen::Dashboard => app.toggle_key_selection(),
            CurrentScreen::KeyContent => pass_to_editor(app, key),
            _ => {}
        },
        KeyCode::Down => match app.current_screen {
            CurrentScreen::Dashboard => app.next_key(),
            CurrentScreen::KeyContent => pass_to_editor(app, key),
            _ => {}
        },
        KeyCode::Up => match app.current_screen {
            CurrentScreen::Dashboard => app.previous_key(),
            CurrentScreen::KeyContent => pass_to_editor(app, key),
            _ => {}
        },

        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_screen == CurrentScreen::KeyContent && !app.is_vim_command_mode {
                pass_to_editor(app, key);
            }
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_screen == CurrentScreen::KeyContent && !app.is_vim_command_mode {
                pass_to_editor(app, key);
            }
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_screen == CurrentScreen::KeyContent && !app.is_vim_command_mode {
                pass_to_editor(app, key);
            }
        }
        KeyCode::Char(':') => {
            // Enter Vim command mode in KeyContent
            if app.current_screen == CurrentScreen::KeyContent && !app.is_vim_command_mode {
                app.is_vim_command_mode = true;
                app.vim_command_input.clear();
            }
        }
        KeyCode::Esc => {
            if app.current_screen == CurrentScreen::KeyContent {
                if app.is_vim_command_mode {
                    app.is_vim_command_mode = false;
                    app.vim_command_input.clear();
                } else {
                    pass_to_editor(app, key);
                }
            }
        }
        KeyCode::Enter => {
            if app.current_screen == CurrentScreen::Dashboard
                && !app.keys.is_empty()
                && app.key_list_state.selected().is_some()
            {
                app.is_loading_value = true;
                terminal.draw(|f| ui::draw(f, app))?;
                app.fetch_value(true).await?;
                app.is_loading_value = false;
            } else if app.current_screen == CurrentScreen::KeyContent {
                if app.is_vim_command_mode {
                    // Execute Vim command
                    let cmd = app.vim_command_input.trim();
                    match cmd {
                        "w" => {
                            // Save
                            app.save_current_value().await?;
                        }
                        "q" => {
                            // Quit (return to Dashboard)
                            app.current_screen = CurrentScreen::Dashboard;
                            app.current_value.clear();
                            app.editor_state = EditorState::default();
                        }
                        "wq" | "x" => {
                            // Save and quit
                            app.save_current_value().await?;
                            app.current_screen = CurrentScreen::Dashboard;
                            app.current_value.clear();
                            app.editor_state = EditorState::default();
                        }
                        "q!" => {
                            // Force quit without saving
                            app.current_screen = CurrentScreen::Dashboard;
                            app.current_value.clear();
                            app.editor_state = EditorState::default();
                        }
                        _ => {
                            // Unknown command, ignore
                        }
                    }
                    app.is_vim_command_mode = false;
                    app.vim_command_input.clear();
                } else {
                    pass_to_editor(app, key);
                }
            }
        }
        KeyCode::Delete | KeyCode::Backspace => {
            if app.current_screen == CurrentScreen::Dashboard && !app.selected_keys.is_empty() {
                app.open_delete_confirmation();
            } else if app.current_screen == CurrentScreen::KeyContent {
                if app.is_vim_command_mode {
                    app.vim_command_input.pop();
                } else {
                    pass_to_editor(app, key);
                }
            }
        }
        KeyCode::Left
        | KeyCode::Right
        | KeyCode::Home
        | KeyCode::End
        | KeyCode::PageUp
        | KeyCode::PageDown
        | KeyCode::Tab => {
            if app.current_screen == CurrentScreen::KeyContent && !app.is_vim_command_mode {
                pass_to_editor(app, key);
            }
        }
        KeyCode::Char(c) => {
            if app.current_screen == CurrentScreen::KeyContent {
                if app.is_vim_command_mode {
                    app.vim_command_input.push(c);
                } else {
                    pass_to_editor(app, key);
                }
            }
        }
        _ => {
            if app.current_screen == CurrentScreen::KeyContent && !app.is_vim_command_mode {
                pass_to_editor(app, key);
            }
        }
    }
    Ok(())
}

async fn handle_file_selector(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    key: KeyEvent,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.current_screen = CurrentScreen::Dashboard;
        }
        KeyCode::Down => {
            app.next_dir_entry();
        }
        KeyCode::Up => {
            app.previous_dir_entry();
        }
        KeyCode::Enter => {
            if let Some(_file_path) = app.enter_selected_entry() {
                // A file was selected, show progress dialog and start import
                app.show_progress_dialog("Import Data".to_string(), "Preparing...".to_string());
                let _ = terminal.draw(|f| ui::draw(f, app));

                // Execute the import
                if let Err(e) = app.import_redis_data().await {
                    app.error_message = Some(format!("Import failed: {}", e));
                }

                // Return to dashboard
                app.current_screen = CurrentScreen::Dashboard;
            }
            // If it was a directory, the method already handled navigation
        }

        _ => {}
    }
    Ok(())
}

async fn handle_connection_switcher(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    key: KeyEvent,
) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.hide_connection_switcher();
        }
        KeyCode::Down => {
            app.next_connection_in_switcher();
        }
        KeyCode::Up => {
            app.previous_connection_in_switcher();
        }
        KeyCode::Enter => {
            terminal.draw(|f| ui::draw(f, app))?;
            app.switch_to_selected_connection().await?;
        }
        KeyCode::Char(c) if c.is_ascii_digit() && app.connection_switcher_search.is_empty() => {
            let num = c.to_digit(10).unwrap() as usize;
            if num > 0 && num <= app.connection_list.connections().len().min(9) {
                app.connection_switcher_state.select(Some(num - 1));
                terminal.draw(|f| ui::draw(f, app))?;
                app.switch_to_selected_connection().await?;
            }
        }
        KeyCode::Char(c) => {
            if !has_modifiers(&key) {
                app.connection_switcher_search.push(c);
                let filtered = app.get_filtered_connections();
                if let Some((idx, _)) = filtered.first() {
                    app.connection_switcher_state.select(Some(*idx));
                }
            }
        }
        _ => {}
    }
    Ok(())
}
