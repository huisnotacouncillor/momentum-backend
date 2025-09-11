use rust_backend::db::models::comment::*;
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Comment功能演示开始");

    // 模拟创建一个评论
    let issue_id = Uuid::new_v4(); // 在实际使用中，这应该是一个真实的issue ID
    let author_id = Uuid::new_v4(); // 在实际使用中，这应该是一个真实的用户ID

    let new_comment = NewComment {
        issue_id,
        author_id,
        content: "这是一个测试评论，支持**Markdown**格式！".to_string(),
        content_type: Some("markdown".to_string()),
        parent_comment_id: None,
    };

    println!("📝 创建新评论...");

    // 注意：这里只是演示数据结构，实际插入需要真实的issue和用户数据
    println!("评论内容: {}", new_comment.content);
    println!("内容类型: {:?}", new_comment.content_type);
    println!("Issue ID: {}", new_comment.issue_id);
    println!("作者 ID: {}", new_comment.author_id);

    // 演示评论提及功能
    let mention = NewCommentMention {
        comment_id: Uuid::new_v4(),        // 评论ID
        mentioned_user_id: Uuid::new_v4(), // 被提及的用户ID
    };

    println!("\n👥 评论提及功能:");
    println!("评论 ID: {}", mention.comment_id);
    println!("被提及用户 ID: {}", mention.mentioned_user_id);

    // 演示附件功能
    let attachment = NewCommentAttachment {
        comment_id: Uuid::new_v4(),
        file_name: "screenshot.png".to_string(),
        file_url: "https://example.com/files/screenshot.png".to_string(),
        file_size: Some(1024 * 1024), // 1MB
        mime_type: Some("image/png".to_string()),
    };

    println!("\n📎 评论附件功能:");
    println!("文件名: {}", attachment.file_name);
    println!("文件URL: {}", attachment.file_url);
    println!("文件大小: {:?} bytes", attachment.file_size);
    println!("MIME类型: {:?}", attachment.mime_type);

    // 演示表情反应功能
    let reaction = NewCommentReaction {
        comment_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        reaction_type: "thumbs_up".to_string(),
    };

    println!("\n👍 评论反应功能:");
    println!("评论 ID: {}", reaction.comment_id);
    println!("用户 ID: {}", reaction.user_id);
    println!("反应类型: {}", reaction.reaction_type);

    // 演示API请求结构
    let create_request = CreateCommentRequest {
        content: "这是通过API创建的评论".to_string(),
        content_type: Some("markdown".to_string()),
        parent_comment_id: None,
        mentions: Some(vec![Uuid::new_v4(), Uuid::new_v4()]),
        attachments: Some(vec![attachment]),
    };

    println!("\n🔗 API请求结构:");
    println!("内容: {}", create_request.content);
    println!(
        "提及用户数量: {:?}",
        create_request.mentions.as_ref().map(|m| m.len())
    );
    println!(
        "附件数量: {:?}",
        create_request.attachments.as_ref().map(|a| a.len())
    );

    println!("\n✅ Comment功能演示完成！");
    println!("\n📋 功能特性总结:");
    println!("  ✓ 基础评论创建和管理");
    println!("  ✓ Markdown内容支持");
    println!("  ✓ 评论回复（嵌套结构）");
    println!("  ✓ @用户提及功能");
    println!("  ✓ 文件附件支持");
    println!("  ✓ 表情反应系统");
    println!("  ✓ 软删除机制");
    println!("  ✓ 权限控制（只能编辑自己的评论）");

    println!("\n🌐 API端点:");
    println!("  GET    /api/issues/:issue_id/comments     - 获取评论列表");
    println!("  POST   /api/issues/:issue_id/comments     - 创建新评论");
    println!("  GET    /api/comments/:comment_id          - 获取单个评论");
    println!("  PUT    /api/comments/:comment_id          - 更新评论");
    println!("  DELETE /api/comments/:comment_id          - 删除评论");
    println!("  POST   /api/comments/:comment_id/reactions - 添加表情反应");
    println!("  DELETE /api/comments/:comment_id/reactions/:type - 移除表情反应");

    Ok(())
}
