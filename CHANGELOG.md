<!-- LTex: Enabled=false -->
# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog.

## [0.0.1] - 

### Added

 - feat(browsers): added view for uninstalled browsers
 - feat(browsers): add setup shortcuts
 - feat(info-page): added an info page
 - feat(browsers): add capabilities to browser info
 - feat(browsers): added some tooltips and info for browser capabilities
 - feat(browsers): added start maximize option
 - feat(icon-picker): added manifest icons + static link /favicon.ico
 - feat: changed name to web app hub
 - feat(browsers): added ungoogled chromium config
 - feat(browsers): added vivaldi config
 - feat(browsers): added issue list for browsers
 - feat(browsers): added floorp config
 - feat(desktop-file): impl Display to DesktopFileError
 - feat(desktop-file): added path checks on load
 - feat(desktop-file): added version + update methods
 - feat(browsers): added profile configs support
 - feat(icon-picker): add throttle to online fetch with 20 sec cache
 - feat(icon-picker): auto select user imported icon
 - feat(web-app-view): url now displays icon fetch + add change icon button error style
 - feat(desktop-file): added custom error to separate errors from validation
 - feat(desktop-file): missing icon with error class
 - feat(web-app-view): remove saved toast + validate input on empty
 - feat(utils): run command methods
 - feat(utils): added log module
 - feat(web-app-view): add error handler
 - feat(browsers): add firefox
 - feat(browsers): add Chrome
 - feat: name change, refactor dev to build.rs, added meta-data
 - feat(app): conditional navigate if user already made apps
 - feat(home): added home page
 - feat(assets): added stand-alone desktop file for desktop icon matching
 - feat(assets): add app icon + refactoring of assets
 - feat(config): added app config based on Cargo.toml
 - feat(icon-picker): picker now handles icon fs saving + cleanup
 - feat(web-app-view): added delete option
 - feat(desktop_file): added profile paths for browsers that should work
 - feat(browser): added base to configs
 - feat(web-app-view): disable isolation row when browser is incapable
 - feat(assets): include assets in the binary and deploy on system
 - feat(error-dialog): added a catch all error dialog
 - feat(web-app-view): added implementation for a new web app
 - feat(icon-picker): failures on dialog can now propagate via failure cb
 - feat(web-apps): implemented fs saving
 - feat(web-app-view): added browser selection
 - feat(web-app-view): add browser label
 - feat(browser-configs): add browser id
 - feat(browsers): implement flatpak installation info
 - feat(icon-picker): implement file picker to add icons
 - feat(browsers): added browsers ui page
 - feat(browser-configs): add icon name support
 - feat(browser-configs): added browser install checks
 - feat(browser-configs): added loadable config files for browsers
 - feat(web-app-view): added isolation ui
 - feat(web-app-view): edit name
 - feat(web-app-view): add undo to icon save toast
 - feat(web-app-view): undo and reset + refactor construction
 - feat(web-app-view): added reset button + desktop file change detection
 - feat(icon-picker): save icons to fs
 - feat(fetch): added error handler
 - feat(fetch): added fetch struct with ureq crate to replace tokio
 - feat(icon-picker): fetch online icons
 - feat(web-apps-view): validate url
 - feat(web-app-view): mut desktop file + url implementation
 - feat(web-app-view): run app button
 - feat(web-apps): added short app name for desktop files
 - feat(web-apps): add no apps found status + err handling
 - feat(web-apps): desktop file paths and file reads
 - feat(pages): added animated navigation for nesting
 - feat(nav-page): added max width
 - feat(ui): added app section
 - feat(pages): icon for nav_page
 - feat(init): init rust adwaita

### Fixed

 - fix(utils): trim run commands
 - fix(browsers): don't show uninstalled browsers when empty
 - fix(web-app-view): set tooltips and sensitivity on browser change
 - fix(desktop_file): use internal method to create profile on checking paths
 - fix(dev): set DISPLAY in container
 - fix(desktop-file): new web apps will validate again with version
 - fix(assets): touch assets file on build so include_dir will update
 - fix(web-app-view): disable 'no browser' selection
 - fix(desktop-file): allow profile config copy when no profile path has been set
 - fix(icon-picker): always reload online icons with reset button
 - fix(web-app-view): change icon button now gets sensitive again after url validation
 - fix(icon-picker): now shows file picker icon when no previous icons found
 - fix(web-app-view): disable save button on invalid and dirty state
 - fix(desktop-file): delete now deletes the profile folder
 - fix(browsers): change firefox profile creation method
 - fix(browsers): chromium browser
 - fix(dev): logging for main app
 - fix: some path fixes with new workspaces
 - fix(dev): don't print errors on dev symlinking
 - fix(desktop-file): follow symlinks when saving
 - fix(fetch): added wget user agent
 - fix(app_dirs): create applications dir before symlink
 - fix(web-app-view): rare parent error on run app button
 - fix(web-app-view): added no-browser option
 - fix(web-app-view): value of isolated is now actually checked
 - fix(web-app-view): reset now properly resets input fields
 - fix(web-app-view): fix remove old file after reset
 - fix(web-app-view): initial browser selection
 - fix(app-menu): config files now reset after user confirm
 - fix(icon-picker): always have a selected icon
 - fix(browsers): flatpak cmd with installation
 - fix(web-apps): web app rows now update changes
 - fix(window): increase size a bit for non scrolling ui
 - fix(web-app-view): run web app arguments are now passed with sh
 - fix(web-apps): dont panic debug build run from other path
 - fix(icon-picker): fixups error logic
 - fix(icon-picker): fix some async issues
 - fix(web-app-view): margin on open button
 - fix(about): use new AboutDialog
 - fix(app): removed weak reference for self declaration
 - fix(nav-page): nav-row is now optional
 - fix(pages): some navigation fixes
 - fix(sidebar): use app name for title
 - fix(view): always display content on navigation

### Changed

 - refactor(utils): command error on run error use response for command errors
 - refactor(browsers-page): rename Browsers to BrowsersPage
 - refactor(browsers): move some logic to desktop-file + profile paths to Path
 - refactor: add workspace for services
 - refactor: replaced xdg with app-dirs struct
 - refactor: use read dir from utils
 - refactor(web-app-view): move profile logic from DesktopFile
 - refactor(desktop-file): removed some more unnessecary parameters
 - refactor(desktop_file): remove unnecessary parameters
 - refactor(web-app-view): use glib for opening apps
 - refactor(desktop-file): refactored desktop-entry ext to desktop-file service
 - refactor(icon-picker): add save method
 - refactor(web-apps): reset app section to method
 - refactor(desktop-entry): remove unnecessary enum
 - refactor: separate logic from ui
 - refactor(browser-configs): convenience and usability
 - refactor(browser-configs): seperate installations
 - refactor(web-app-view): some optimisations + more debug logging
 - refactor(web-app-view): update icon build to own method
 - refactor(icon-picker): renamed loading icons method
 - refactor: some pub / priv changes
 - refactor(pages): remove page suffix from files
 - refactor(clippy): pedantic fixes

[Unreleased]: https://github.com/PVermeer/web-app-hub/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/PVermeer/web-app-hub/releases/tag/v0.0.1

