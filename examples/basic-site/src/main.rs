use luperiq::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = LuperiqApp::builder()
        .site_name("My Basic Site")
        .base_url("https://example.com")
        .with_module(BlogModule::builder()
            .posts_dir("./content/posts")
            .build())
        .with_module(PageModule::builder()
            .pages_dir("./content/pages")
            .build())
        .with_module(SeoModule::builder()
            .sitemap(true)
            .structured_data(true)
            .build())
        .with_module(FormModule::builder()
            .endpoint("/api/forms")
            .build())
        .with_module(DirectoryModule::default())
        .with_theme("default")
        .output_dir("./output")
        .build()?;

    app.load_content().await?;

    app.render_page("/", PageConfig {
        title: "Home".into(),
        template: "home".into(),
        ..Default::default()
    }).await?;

    app.render_blog("/blog", BlogConfig {
        posts_per_page: 10,
        ..Default::default()
    }).await?;

    app.render_directory("/directory", DirectoryConfig {
        categories: vec!["services".into(), "products".into()],
        ..Default::default()
    }).await?;

    app.generate_sitemap().await?;
    app.generate_rss_feed("/blog/feed.xml").await?;

    let report = app.generate_report().await?;
    println!("Generated {} pages, {} posts, {} directory listings",
        report.pages, report.posts, report.listings);

    Ok(())
}
