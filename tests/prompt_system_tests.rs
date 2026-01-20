use std::fs;

#[test]
fn test_prompt_command_exists() {
    let plugins_src = fs::read_to_string("src/app/plugins.rs").expect("Failed to read plugins.rs");

    // Verify Prompt command exists in PluginCommand enum
    assert!(
        plugins_src.contains("Prompt {"),
        "PluginCommand should have Prompt variant"
    );

    // Verify required fields
    assert!(plugins_src.contains("id: String"), "Prompt should have id field");
    assert!(
        plugins_src.contains("title: String"),
        "Prompt should have title field"
    );
    assert!(
        plugins_src.contains("message: Option<String>"),
        "Prompt should have optional message field"
    );
    assert!(
        plugins_src.contains("default: Option<String>"),
        "Prompt should have optional default field"
    );
}

#[test]
fn test_confirm_command_exists() {
    let plugins_src = fs::read_to_string("src/app/plugins.rs").expect("Failed to read plugins.rs");

    // Verify Confirm command exists in PluginCommand enum
    assert!(
        plugins_src.contains("Confirm {"),
        "PluginCommand should have Confirm variant"
    );

    // Verify required fields
    assert!(
        plugins_src.contains("ok_label: Option<String>"),
        "Confirm should have optional ok_label field"
    );
    assert!(
        plugins_src.contains("cancel_label: Option<String>"),
        "Confirm should have optional cancel_label field"
    );
}

#[test]
fn test_pick_command_exists() {
    let plugins_src = fs::read_to_string("src/app/plugins.rs").expect("Failed to read plugins.rs");

    // Verify Pick command exists in PluginCommand enum
    assert!(
        plugins_src.contains("Pick {"),
        "PluginCommand should have Pick variant"
    );

    // Verify required fields
    assert!(
        plugins_src.contains("items: Vec<String>"),
        "Pick should have items field"
    );
    assert!(
        plugins_src.contains("selected: Option<usize>"),
        "Pick should have optional selected field"
    );
}

#[test]
fn test_prompt_response_event_exists() {
    let plugins_src = fs::read_to_string("src/app/plugins.rs").expect("Failed to read plugins.rs");

    // Verify prompt_response event is handled
    assert!(
        plugins_src.contains("prompt_response"),
        "Plugin system should handle prompt_response events"
    );

    // Verify dispatch_prompt_response function exists
    assert!(
        plugins_src.contains("dispatch_prompt_response"),
        "Should have dispatch_prompt_response function"
    );
}

#[test]
fn test_prompt_owners_tracking() {
    let plugins_src = fs::read_to_string("src/app/plugins.rs").expect("Failed to read plugins.rs");

    // Verify prompt owners are tracked
    assert!(
        plugins_src.contains("prompt_owners"),
        "Should track prompt owners for response routing"
    );

    // Verify prompt owners are used for response routing
    assert!(
        plugins_src.contains("prompt_owners.insert"),
        "Should insert prompt owner when creating prompt"
    );
    assert!(
        plugins_src.contains("prompt_owners.remove"),
        "Should remove prompt owner when handling response"
    );
}

#[test]
fn test_prompt_command_handler() {
    let plugins_src = fs::read_to_string("src/app/plugins.rs").expect("Failed to read plugins.rs");

    // Verify Prompt command is handled
    assert!(
        plugins_src.contains("PluginCommand::Prompt"),
        "Should handle Prompt command"
    );

    // Verify it opens a prompt
    assert!(
        plugins_src.contains("open_prompt") || plugins_src.contains("PromptState"),
        "Should open prompt UI"
    );
}

#[test]
fn test_confirm_command_handler() {
    let plugins_src = fs::read_to_string("src/app/plugins.rs").expect("Failed to read plugins.rs");

    // Verify Confirm command is handled
    assert!(
        plugins_src.contains("PluginCommand::Confirm"),
        "Should handle Confirm command"
    );
}

#[test]
fn test_pick_command_handler() {
    let plugins_src = fs::read_to_string("src/app/plugins.rs").expect("Failed to read plugins.rs");

    // Verify Pick command is handled
    assert!(
        plugins_src.contains("PluginCommand::Pick"),
        "Should handle Pick command"
    );
}

#[test]
fn test_worktree_uses_prompts() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify worktree plugin uses prompt for new branch
    assert!(
        plugin_src.contains("emit_prompt"),
        "Worktree should use prompt for branch name"
    );

    // Verify it uses confirm for close operation
    assert!(
        plugin_src.contains("emit_confirm"),
        "Worktree should use confirm for close operation"
    );

    // Verify it uses pick for switch/close selection
    assert!(
        plugin_src.contains("emit_pick"),
        "Worktree should use pick for worktree selection"
    );
}

#[test]
fn test_prompt_json_structure() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify emit_prompt generates correct JSON
    assert!(
        plugin_src.contains(r#""command":"prompt""#),
        "emit_prompt should set command to 'prompt'"
    );
    assert!(
        plugin_src.contains(r#""id":"%s""#),
        "emit_prompt should include id field"
    );
    assert!(
        plugin_src.contains(r#""title":"%s""#),
        "emit_prompt should include title field"
    );
}

#[test]
fn test_confirm_json_structure() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify emit_confirm generates correct JSON
    assert!(
        plugin_src.contains(r#""command":"confirm""#),
        "emit_confirm should set command to 'confirm'"
    );
    assert!(
        plugin_src.contains(r#""ok_label":"%s""#),
        "emit_confirm should include ok_label field"
    );
    assert!(
        plugin_src.contains(r#""cancel_label":"%s""#),
        "emit_confirm should include cancel_label field"
    );
}

#[test]
fn test_pick_json_structure() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify emit_pick generates correct JSON
    assert!(
        plugin_src.contains(r#""command":"pick""#),
        "emit_pick should set command to 'pick'"
    );
    assert!(
        plugin_src.contains(r#""items":%s"#),
        "emit_pick should include items field"
    );
}

#[test]
fn test_prompt_response_data_handling() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify prompt response handling
    assert!(
        plugin_src.contains("data.ok"),
        "Should check if user confirmed or cancelled"
    );
    assert!(
        plugin_src.contains("data.value"),
        "Should extract user input value from prompt"
    );
    assert!(
        plugin_src.contains("data.index"),
        "Should extract selected index from pick"
    );
}

#[test]
fn test_prompt_id_tracking() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify ID generation for prompts
    assert!(
        plugin_src.contains("req_id="),
        "Should generate request IDs for prompts"
    );
    assert!(
        plugin_src.contains("date +%s%N"),
        "Should use timestamp for unique IDs"
    );

    // Verify state is saved with ID
    assert!(
        plugin_src.contains("save_request"),
        "Should save request state with ID"
    );

    // Verify state is loaded by ID
    assert!(
        plugin_src.contains("load_request"),
        "Should load request state by ID"
    );
}

#[test]
fn test_prompt_cancellation_handling() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify cancellation is handled
    assert!(
        plugin_src.contains(r#"[ "$ok" != "True" ]"#),
        "Should check if prompt was cancelled"
    );

    // Verify cleanup on cancellation
    assert!(
        plugin_src.contains("rm_request"),
        "Should clean up state when prompt is cancelled"
    );
}

#[test]
fn test_json_escaping_in_prompts() {
    let plugin_src =
        fs::read_to_string("examples/plugins/worktree/plugin.sh").expect("Failed to read plugin");

    // Verify JSON escaping is used
    assert!(
        plugin_src.contains("json_escape"),
        "Should use json_escape for user-provided strings"
    );

    // Verify it's used in all emit functions
    assert!(
        plugin_src.contains("id_esc="),
        "Should escape ID before including in JSON"
    );
    assert!(
        plugin_src.contains("title_esc="),
        "Should escape title before including in JSON"
    );
    assert!(
        plugin_src.contains("msg_esc="),
        "Should escape message before including in JSON"
    );
}
