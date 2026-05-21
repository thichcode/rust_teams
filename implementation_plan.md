# Implementation Plan

[Overview]
Create a lightweight Teams Lite Wrapper application that provides core Teams functionality with minimal resource usage. The application will wrap Microsoft Teams Web in a native window with additional features like tray icon, auto-login, notifications, and profile management. Target RAM usage: 100-250MB idle, 300-800MB when Teams is active.

This implementation addresses the need for a lighter alternative to the official Teams desktop client while maintaining essential features. The application will use WebView2 for rendering Teams Web, providing native integration with better performance than a full browser. The solution fits into the existing system as a standalone desktop application that can coexist with or replace the official Teams client.

[Types]

## Core Data Structures

### Profile Types
```rust
struct Profile {
    id: String,
    name: String, // "Work", "Admin", "Personal"
    teams_url: String, // Default: "https://teams.microsoft.com"
    cookies: HashMap<String, String>,
    session_data: Option<SessionData>,
    auto_login: bool,
    is_default: bool,
}

struct SessionData {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: DateTime<Utc>,
    user_info: UserInfo,
}

struct UserInfo {
    display_name: String,
    email: String,
    avatar_url: Option<String>,
}
```

### Configuration Types
```rust
struct AppConfig {
    profiles: Vec<Profile>,
    current_profile_id: Option<String>,
    window_settings: WindowSettings,
    notification_settings: NotificationSettings,
    privacy_settings: PrivacySettings,
    shortcuts: HashMap<String, String>,
}

struct WindowSettings {
    width: u32,
    height: u32,
    x: Option<i32>,
    y: Option<i32>,
    maximized: bool,
    always_on_top: bool,
    transparent: bool,
}

struct NotificationSettings {
    enabled: bool,
    show_badge: bool,
    play_sound: bool,
    toast_duration: u64, // milliseconds
}

struct PrivacySettings {
    block_ads: bool,
    block_tracking: bool,
    blocked_domains: Vec<String>,
    clear_cookies_on_exit: bool,
}
```

### Notification Types
```rust
enum NotificationType {
    Message,
    Call,
    Meeting,
    Mention,
    Reaction,
}

struct AppNotification {
    id: String,
    title: String,
    body: String,
    notification_type: NotificationType,
    timestamp: DateTime<Utc>,
    profile_id: Option<String>,
    is_read: bool,
    action_url: Option<String>,
}
```

### WebView Types
```rust
enum WebViewAction {
    Navigate(String),
    GoBack,
    GoForward,
    Reload,
    OpenDevTools,
    CaptureScreenshot,
}

struct WebViewState {
    current_url: String,
    can_go_back: bool,
    can_go_forward: bool,
    is_loading: bool,
    title: Option<String>,
    favicon: Option<Vec<u8>>,
}
```

### Tray Icon Types
```rust
enum TrayMenuItem {
    ShowWindow,
    HideWindow,
    NewProfile,
    SwitchProfile(String),
    Settings,
    Quit,
}

struct TrayIconState {
    visible: bool,
    menu_items: Vec<TrayMenuItem>,
    tooltip: String,
    icon: TrayIconType,
}

enum TrayIconType {
    Normal,
    Unread,
    Call,
    Meeting,
}
```

[Files]

## New Files to be Created

### Main Application Files
- `src/main.rs` - Entry point and application initialization
- `src/app.rs` - Main application state and logic
- `src/config.rs` - Configuration management (load/save/validate)
- `src/error.rs` - Custom error types and handling

### UI and Window Management
- `src/ui/mod.rs` - UI module exports
- `src/ui/window.rs` - Main application window management
- `src/ui/tray.rs` - System tray icon implementation
- `src/ui/webview.rs` - WebView2 wrapper and management
- `src/ui/notification.rs` - Desktop notification system

### Core Functionality
- `src/teams/mod.rs` - Teams-specific functionality
- `src/teams/auth.rs` - Auto-login and session management
- `src/teams/url_handler.rs` - URL routing and external link handling
- `src/teams/blocker.rs` - Ad/tracking domain blocking

### Profile Management
- `src/profiles/mod.rs` - Profile management module
- `src/profiles/manager.rs` - Profile CRUD operations
- `src/profiles/switcher.rs` - Profile switching logic

### Utilities
- `src/utils/mod.rs` - Utility functions
- `src/utils/shortcuts.rs` - Keyboard shortcut handling
- `src/utils/platform.rs` - Platform-specific utilities
- `src/utils/logging.rs` - Logging configuration

### Configuration Files
- `config/default_config.json` - Default configuration template
- `config/blocked_domains.json` - Default blocked domains list
- `Cargo.toml` - Rust project configuration
- `.gitignore` - Git ignore patterns

## File Structure
```
rust_teams/
├── Cargo.toml
├── .gitignore
├── config/
│   ├── default_config.json
│   └── blocked_domains.json
└── src/
    ├── main.rs
    ├── app.rs
    ├── config.rs
    ├── error.rs
    ├── ui/
    │   ├── mod.rs
    │   ├── window.rs
    │   ├── tray.rs
    │   ├── webview.rs
    │   └── notification.rs
    ├── teams/
    │   ├── mod.rs
    │   ├── auth.rs
    │   ├── url_handler.rs
    │   └── blocker.rs
    ├── profiles/
    │   ├── mod.rs
    │   ├── manager.rs
    │   └── switcher.rs
    └── utils/
        ├── mod.rs
        ├── shortcuts.rs
        ├── platform.rs
        └── logging.rs
```

[Functions]

## Main Application Functions

### Entry Point (main.rs)
- `main()` - Application entry point with error handling
- `build_app()` - Create and configure the main application

### Application State (app.rs)
- `App::new(config: AppConfig) -> Result<Self>` - Create new application instance
- `App::run(&mut self) -> Result<()>` - Main application event loop
- `App::handle_event(&mut self, event: AppEvent) -> Result<()>` - Handle application events
- `App::initialize(&mut self) -> Result<()>` - Initialize all components
- `App::shutdown(&mut self) -> Result<()>` - Clean shutdown of all components

### Configuration Management (config.rs)
- `ConfigManager::new() -> Self` - Create configuration manager
- `ConfigManager::load() -> Result<AppConfig>` - Load configuration from file
- `ConfigManager::save(config: &AppConfig) -> Result<()>` - Save configuration to file
- `ConfigManager::validate(config: &AppConfig) -> Result<()>` - Validate configuration
- `ConfigManager::get_config_path() -> PathBuf` - Get configuration file path
- `ConfigManager::reset_to_defaults() -> Result<()>` - Reset to default configuration

### Window Management (ui/window.rs)
- `WindowManager::new() -> Result<Self>` - Create window manager
- `WindowManager::create_window(&self, settings: &WindowSettings) -> Result<()>` - Create main window
- `WindowManager::show(&self) -> Result<()>` - Show the window
- `WindowManager::hide(&self) -> Result<()>` - Hide the window
- `WindowManager::toggle(&self) -> Result<()>` - Toggle window visibility
- `WindowManager::set_always_on_top(&self, enabled: bool) -> Result<()>` - Set always on top
- `WindowManager::set_size(&self, width: u32, height: u32) -> Result<()>` - Resize window
- `WindowManager::set_position(&self, x: i32, y: i32) -> Result<()>` - Move window
- `WindowManager::maximize(&self) -> Result<()>` - Maximize window
- `WindowManager::restore(&self) -> Result<()>` - Restore window
- `WindowManager::close(&self) -> Result<()>` - Close window

### WebView Management (ui/webview.rs)
- `WebViewManager::new(window: &Window) -> Result<Self>` - Create WebView2 instance
- `WebViewManager::initialize(&mut self) -> Result<()>` - Initialize WebView2 environment
- `WebViewManager::navigate(&self, url: &str) -> Result<()>` - Navigate to URL
- `WebViewManager::execute_script(&self, script: &str) -> Result<()>` - Execute JavaScript
- `WebViewManager::go_back(&self) -> Result<()>` - Navigate back
- `WebViewManager::go_forward(&self) -> Result<()>` - Navigate forward
- `WebViewManager::reload(&self) -> Result<()>` - Reload current page
- `WebViewManager::open_dev_tools(&self) -> Result<()>` - Open developer tools
- `WebViewManager::capture_screenshot(&self) -> Result<Vec<u8>>` - Capture screenshot
- `WebViewManager::get_title(&self) -> Option<String>` - Get current page title
- `WebViewManager::get_url(&self) -> String` - Get current URL
- `WebViewManager::set_cookies(&self, cookies: &HashMap<String, String>) -> Result<()>` - Set cookies
- `WebViewManager::get_cookies(&self, url: &str) -> Result<HashMap<String, String>>` - Get cookies for URL
- `WebViewManager::clear_cookies(&self) -> Result<()>` - Clear all cookies
- `WebViewManager::add_navigation_handler(&mut self, handler: Box<dyn NavigationHandler>) -> Result<()>` - Add navigation event handler
- `WebViewManager::add_message_handler(&mut self, handler: Box<dyn MessageHandler>) -> Result<()>` - Add message handler

### Tray Icon (ui/tray.rs)
- `TrayIcon::new() -> Result<Self>` - Create tray icon
- `TrayIcon::set_icon(&self, icon_type: TrayIconType) -> Result<()>` - Set tray icon
- `TrayIcon::set_tooltip(&self, tooltip: &str) -> Result<()>` - Set tooltip text
- `TrayIcon::update_menu(&self, menu_items: &[TrayMenuItem]) -> Result<()>` - Update context menu
- `TrayIcon::show(&self) -> Result<()>` - Show tray icon
- `TrayIcon::hide(&self) -> Result<()>` - Hide tray icon
- `TrayIcon::set_visible(&self, visible: bool) -> Result<()>` - Set visibility
- `TrayIcon::add_handler(&mut self, handler: Box<dyn TrayEventHandler>) -> Result<()>` - Add event handler

### Notification System (ui/notification.rs)
- `NotificationManager::new() -> Result<Self>` - Create notification manager
- `NotificationManager::show_notification(&self, notification: AppNotification) -> Result<()>` - Show desktop notification
- `NotificationManager::show_toast(&self, title: &str, body: &str) -> Result<()>` - Show simple toast notification
- `NotificationManager::update_badge(&self, count: u32) -> Result<()>` - Update app badge count
- `NotificationManager::clear_notifications(&self) -> Result<()>` - Clear all notifications
- `NotificationManager::mark_as_read(&self, notification_id: &str) -> Result<()>` - Mark notification as read
- `NotificationManager::add_handler(&mut self, handler: Box<dyn NotificationHandler>) -> Result<()>` - Add notification click handler

### Authentication (teams/auth.rs)
- `AuthManager::new(profile: &Profile) -> Self` - Create auth manager for profile
- `AuthManager::auto_login(&self, webview: &WebViewManager) -> Result<()>` - Perform auto-login
- `AuthManager::save_session(&self, session: SessionData) -> Result<()>` - Save session data
- `AuthManager::load_session(&self) -> Result<Option<SessionData>>` - Load saved session
- `AuthManager::refresh_token(&self) -> Result<()>` - Refresh access token
- `AuthManager::clear_session(&self) -> Result<()>` - Clear session data
- `AuthManager::is_logged_in(&self) -> bool` - Check if user is logged in
- `AuthManager::get_user_info(&self) -> Result<Option<UserInfo>>` - Get current user info

### URL Handler (teams/url_handler.rs)
- `UrlHandler::new() -> Self` - Create URL handler
- `UrlHandler::handle_navigation(&self, url: &str) -> NavigationDecision` - Handle navigation requests
- `UrlHandler::open_external(&self, url: &str) -> Result<()>` - Open URL in default browser
- `UrlHandler::is_external(&self, url: &str) -> bool` - Check if URL should open externally
- `UrlHandler::is_teams_url(&self, url: &str) -> bool` - Check if URL is Teams-related

### Domain Blocker (teams/blocker.rs)
- `DomainBlocker::new() -> Self` - Create domain blocker
- `DomainBlocker::load_blocked_domains(&mut self, path: Option<&str>) -> Result<()>` - Load blocked domains
- `DomainBlocker::is_blocked(&self, url: &str) -> bool` - Check if URL is blocked
- `DomainBlocker::add_blocked_domain(&mut self, domain: &str) -> Result<()>` - Add domain to block list
- `DomainBlocker::remove_blocked_domain(&mut self, domain: &str) -> Result<()>` - Remove domain from block list
- `DomainBlocker::get_blocked_domains(&self) -> &Vec<String>` - Get all blocked domains

### Profile Management (profiles/manager.rs)
- `ProfileManager::new() -> Self` - Create profile manager
- `ProfileManager::load_profiles(&mut self) -> Result<()>` - Load all profiles
- `ProfileManager::save_profiles(&self) -> Result<()>` - Save all profiles
- `ProfileManager::create_profile(&mut self, profile: Profile) -> Result<()>` - Create new profile
- `ProfileManager::delete_profile(&mut self, profile_id: &str) -> Result<()>` - Delete profile
- `ProfileManager::switch_profile(&mut self, profile_id: &str) -> Result<()>` - Switch to profile
- `ProfileManager::get_current_profile(&self) -> Option<&Profile>` - Get current profile
- `ProfileManager::get_profile(&self, profile_id: &str) -> Option<&Profile>` - Get profile by ID
- `ProfileManager::update_profile(&mut self, profile: Profile) -> Result<()>` - Update profile
- `ProfileManager::get_profiles(&self) -> &Vec<Profile>` - Get all profiles

### Keyboard Shortcuts (utils/shortcuts.rs)
- `ShortcutManager::new() -> Self` - Create shortcut manager
- `ShortcutManager::register_shortcut(&mut self, key: &str, handler: Box<dyn Fn()>) -> Result<()>` - Register keyboard shortcut
- `ShortcutManager::unregister_shortcut(&mut self, key: &str) -> Result<()>` - Unregister shortcut
- `ShortcutManager::handle_key_event(&self, event: &KeyEvent) -> bool` - Handle key events
- `ShortcutManager::load_default_shortcuts(&mut self) -> Result<()>` - Load default shortcuts

[Classes]

## Main Classes/Structs

### Application Core
- **App** (src/app.rs) - Main application struct containing all managers and state
  - Key methods: new(), run(), handle_event(), initialize(), shutdown()
  - Inherits/uses: WindowManager, WebViewManager, TrayIcon, NotificationManager, ProfileManager, AuthManager

### UI Components
- **WindowManager** (src/ui/window.rs) - Manages the main application window
  - Key methods: create_window(), show(), hide(), toggle(), set_always_on_top(), set_size(), set_position()
  - Uses: winit for window management

- **WebViewManager** (src/ui/webview.rs) - Manages WebView2 instance
  - Key methods: initialize(), navigate(), execute_script(), go_back(), go_forward(), reload()
  - Uses: webview2-com for WebView2 integration

- **TrayIcon** (src/ui/tray.rs) - System tray icon implementation
  - Key methods: set_icon(), set_tooltip(), update_menu(), show(), hide()
  - Uses: tray-item for cross-platform tray icon

- **NotificationManager** (src/ui/notification.rs) - Desktop notification system
  - Key methods: show_notification(), show_toast(), update_badge(), clear_notifications()
  - Uses: notify-rust for cross-platform notifications

### Teams Functionality
- **AuthManager** (src/teams/auth.rs) - Handles authentication and session management
  - Key methods: auto_login(), save_session(), load_session(), refresh_token(), clear_session()
  - Uses: cookies and session storage

- **UrlHandler** (src/teams/url_handler.rs) - URL routing and external link handling
  - Key methods: handle_navigation(), open_external(), is_external(), is_teams_url()
  - Uses: url parsing libraries

- **DomainBlocker** (src/teams/blocker.rs) - Blocks ads and tracking domains
  - Key methods: is_blocked(), add_blocked_domain(), remove_blocked_domain()
  - Uses: regex for URL matching

### Profile Management
- **ProfileManager** (src/profiles/manager.rs) - Manages user profiles
  - Key methods: create_profile(), delete_profile(), switch_profile(), get_current_profile()
  - Uses: serde for serialization

### Utilities
- **ConfigManager** (src/config.rs) - Configuration management
  - Key methods: load(), save(), validate(), reset_to_defaults()
  - Uses: serde_json for JSON serialization

- **ShortcutManager** (src/utils/shortcuts.rs) - Keyboard shortcut handling
  - Key methods: register_shortcut(), unregister_shortcut(), handle_key_event()
  - Uses: winit for keyboard input

[Dependencies]

## Rust Crates

### Core Dependencies
- **webview2-com** (0.20.0) - WebView2 COM bindings for Windows
- **winit** (0.29.0) - Cross-platform window creation and management
- **tray-item** (0.10.0) - System tray icon support
- **notify-rust** (4.8.0) - Desktop notifications
- **serde** (1.0) with serde_json - JSON serialization/deserialization
- **tokio** (1.0) - Async runtime for background tasks
- **anyhow** (1.0) - Error handling
- **thiserror** (1.0) - Custom error types
- **log** (0.4) with env_logger - Logging framework
- **once_cell** (1.0) - Lazy static initialization
- **parking_lot** (0.12) - Thread-safe primitives
- **uuid** (1.0) - UUID generation for IDs
- **chrono** (0.4) - Date/time handling
- **url** (2.0) - URL parsing
- **regex** (1.0) - Regular expressions for domain blocking
- **directories** (4.0) - Platform-specific directories (config, cache, etc.)
- **image** (0.24) - Image processing for screenshots
- **clipboard** (0.5) - Clipboard access
- **open** (5.0) - Open URLs in default browser

### Dev Dependencies
- **mockall** (0.11) - Mocking for tests
- **tempfile** (3.0) - Temporary files for tests

### Build Dependencies
- **windows-bindgen** - For Windows-specific bindings if needed

## Native Dependencies
- **WebView2 Runtime** - Required on Windows (will be auto-installed if not present)

## Version Requirements
- Rust: 1.70.0 or higher (for async/await and other modern features)
- Windows: 10 or 11 (WebView2 requires Windows 10 version 1803 or later)

[Testing]

## Testing Strategy

### Unit Tests
- Configuration loading/saving/validation
- Profile creation/deletion/switching
- Domain blocking logic
- URL parsing and handling
- Session management

### Integration Tests
- WebView2 initialization and navigation
- Window creation and management
- Tray icon functionality
- Notification display
- Keyboard shortcut handling

### Test Files
- `tests/config_test.rs` - Configuration tests
- `tests/profiles_test.rs` - Profile management tests
- `tests/blocker_test.rs` - Domain blocker tests
- `tests/url_handler_test.rs` - URL handling tests
- `tests/integration_test.rs` - Integration tests (may require WebView2)

### Mocking
- Mock WebView2 for unit tests
- Mock window system for UI tests
- Mock notification system

### Test Coverage
- Aim for 80%+ code coverage
- Focus on critical paths: auth, navigation, profile switching
- Test edge cases: network errors, invalid URLs, missing cookies

### CI/CD
- GitHub Actions workflow for testing on Windows
- Test matrix: stable Rust, Windows latest
- Artifact building for releases

[Implementation Order]

1. **Project Setup and Configuration**
   - Create Cargo.toml with all dependencies
   - Set up project structure and directories
   - Create default configuration files
   - Implement error handling and logging

2. **Core Infrastructure**
   - Implement configuration management (config.rs)
   - Create main application struct (app.rs)
   - Set up logging and error handling
   - Create utility functions

3. **Window and WebView Foundation**
   - Implement window management (ui/window.rs)
   - Set up WebView2 environment and basic navigation (ui/webview.rs)
   - Create main.rs entry point
   - Test basic window with WebView2

4. **Profile Management System**
   - Implement profile types and storage (profiles/mod.rs, profiles/manager.rs)
   - Create profile switching logic (profiles/switcher.rs)
   - Integrate profiles with main app
   - Test profile creation and switching

5. **Authentication and Session Management**
   - Implement auto-login with cookies (teams/auth.rs)
   - Create session management
   - Integrate with WebView2
   - Test login flow

6. **URL Handling and External Links**
   - Implement URL handler (teams/url_handler.rs)
   - Add external link detection
   - Implement browser opening for external links
   - Test URL routing

7. **Domain Blocking**
   - Implement domain blocker (teams/blocker.rs)
   - Load default blocked domains
   - Integrate with WebView2 navigation
   - Test blocking functionality

8. **Tray Icon and Notifications**
   - Implement tray icon (ui/tray.rs)
   - Create notification system (ui/notification.rs)
   - Add notification handlers
   - Test tray and notifications

9. **Keyboard Shortcuts**
   - Implement shortcut manager (utils/shortcuts.rs)
   - Add default shortcuts
   - Integrate with app events
   - Test shortcut handling

10. **Final Integration and Testing**
    - Integrate all components
    - Add comprehensive error handling
    - Write integration tests
    - Optimize performance
    - Test RAM usage targets

11. **Polishing and Documentation**
    - Add user documentation
    - Create README with usage instructions
    - Add build scripts
    - Final testing and validation