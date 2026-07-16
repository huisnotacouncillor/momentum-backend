//! Issue #13 守门：dead `events/` 模块、batch_processor、unified_manager
//! 已被彻底删除（之前在 `pub mod events;` 注释里 — 是从未启用的半完成
//! 重构）。此测试防止有人重新启用未审计的死代码。

#[cfg(test)]
mod dead_code_guard_tests {
    /// mod.rs 不能再次声明已弃用的模块
    /// （这条检查必须排除 dead_code_guard_test 自身，因为它会包含
    ///  "pub mod events;" 字面量讨论）
    #[test]
    fn websocket_mod_does_not_redeclare_dead_modules() {
        let modules_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/websocket");
        // 直接从文件系统读取 mod.rs，绕开本测试文件
        let mod_path = format!("{}/mod.rs", modules_dir);
        let source = std::fs::read_to_string(&mod_path).expect("mod.rs must exist");
        assert!(
            !source.contains("pub mod events;"),
            "websocket::events is dead code (4 TODO permission checks). \
             Do not re-enable without first completing all RBAC checks."
        );
        assert!(
            !source.contains("pub mod batch_processor;"),
            "batch_processor is dead code. Do not re-enable."
        );
        assert!(
            !source.contains("pub mod unified_manager;"),
            "unified_manager is dead code. Do not re-enable."
        );
    }

    /// 文件系统层：dead 模块目录不应存在
    #[test]
    fn dead_event_files_dont_exist_on_disk() {
        // 把 src/ 当成根
        let modules_dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/websocket"
        );
        let mut found = Vec::new();
        for entry in std::fs::read_dir(modules_dir)
            .expect("websocket modules dir must exist")
            .flatten()
        {
            let name = entry.file_name().into_string().unwrap_or_default();
            if name == "events"
                || name == "batch_processor.rs"
                || name == "unified_manager.rs"
            {
                found.push(name);
            }
        }
        assert!(
            found.is_empty(),
            "Issue #13 dead code must stay removed. Found: {:?}",
            found
        );
    }

    /// 守门：原本 dead code 中含 4 个 TODO permission 检查的字符串不应再出现
    /// 排除本测试文件本身（含 "TODO: 实现" 字符串）
    #[test]
    fn no_more_todo_permission_check_strings() {
        let modules_dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/websocket"
        );
        let test_file = file!();
        let todo_count: u32 = std::fs::read_dir(modules_dir)
            .expect("modules dir")
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                let path_str = p.to_string_lossy().to_string();
                // 不要把守门测试自己算进去（它讨论 TODO）
                if path_str.ends_with(test_file) {
                    return None;
                }
                if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                    Some(p)
                } else {
                    None
                }
            })
            .map(|p| {
                std::fs::read_to_string(&p)
                    .unwrap_or_default()
                    .matches("TODO: 实现")
                    .count() as u32
            })
            .sum();
        assert_eq!(
            todo_count, 0,
            "websocket/ contains {} 'TODO: 实现' (Issue #13 dead-code markers). \
             Author proper logic instead.",
            todo_count
        );
    }
}
