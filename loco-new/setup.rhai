// =====================
// Base Files
// =====================
// Copy core project structure files and directories that are fundamental
// to the Rust environment, GitHub actions, and formatting settings.

gen.copy_template(".github/workflows/ci.yaml.t");     // Actions ci template
gen.copy_files([".gitignore", ".rustfmt.toml", "README.md"]);

gen.copy_template_dir(".cargo");

// =====================
// Core Source Files
// =====================
gen.copy_template("src/controllers/mod.rs.t");      // Main controller module template
gen.copy_template("src/views/mod.rs.t");            // Main views module template
gen.copy_template("src/tasks/mod.rs.t");            // Main tasks module template

gen.copy_template("src/initializers/mod.rs.t");     // initializer module

// Main application and library templates
gen.copy_template("src/app.rs.t");                  // App root file
gen.copy_template("src/lib.rs.t");                  // Library entry file
gen.copy_template("Cargo.toml.t");                  // Project’s cargo configuration

// bin
gen.copy_template("src/bin/main.rs.t");
if windows {
  gen.copy_template("src/bin/tool.rs.t");
}


// =====================
// Test Files
// =====================
// Generates and organizes tests modules and templates for different areas of the application.

gen.copy_template("tests/mod.rs.t");                // Main tests module template
gen.copy_template("tests/requests/mod.rs.t");       // HTTP requests tests module
gen.copy_template("tests/tasks/mod.rs.t");          // Tasks tests module


// =====================
// App Configuration
// =====================
gen.copy_template("config/development.yaml.t");     // Development config template
gen.copy_template("config/test.yaml.t");            // Test config template
gen.copy_file("config/production.yaml");            // Production config

// =====================
// Database-Related Files
// =====================
if db {
   // Database migrations configuration and setup
    gen.copy_template("migration/Cargo.toml.t");                  // Database migrations Cargo configuration
    gen.copy_template("migration/src/lib.rs.t");                  // Database migrations library

    // Entity modules for database models
    gen.copy_template("src/models/_entities/mod.rs.t");            // Root module for database entities
    gen.copy_template("src/models/_entities/prelude.rs.t");            // Root module for database entities
    gen.copy_template("src/models/mod.rs.t");            // Root module for database entities

    // Test modules related to database models
    gen.copy_template("tests/models/mod.rs.t");                   // Models tests root module

    if (settings.auth) {
        // Authentication-related models and migrations
        gen.copy_file("migration/src/m20220101_000001_users.rs");  // Users migration file
        gen.copy_file("src/models/_entities/users.rs");             // Users entity definition
        gen.copy_file("src/models/users.rs");                      // Users model logic

        // Fixtures and test setup for authentication
        gen.copy_dir("src/fixtures");                              // Database fixtures directory

        // Test modules related to user models
        gen.copy_dir("tests/models/snapshots");                    // Test snapshots for models
        gen.copy_template("tests/models/users.rs.t");              // User model test template
        gen.copy_template("tests/requests/prepare_data.rs.t");     // Data preparation template for tests
    }
   
   gen.copy_template("examples/playground.rs.t");                 // Example playground template with DB setup
}

// =====================
// Initializers Support
// =====================
if settings.initializers.view_engine {
   gen.copy_file("src/initializers/view_engine.rs"); // Template for view engine initializer
}

// =====================
// Authentication Setup
// =====================
if settings.auth {
   gen.copy_file("src/controllers/auth.rs");        // Auth controller
   gen.copy_file("src/views/auth.rs");              // Auth views

   gen.copy_template("tests/requests/auth.rs.t");   // Auth tests template
   gen.copy_dir("tests/requests/snapshots");        // Snapshots directory for auth tests
} 
else {
   gen.copy_file("src/controllers/home.rs");        // Home controller if auth not enabled
   gen.copy_file("src/views/home.rs");              // Home views
   gen.copy_template("tests/requests/home.rs.t");   // Home tests template
}

// =====================
// Mailer Setup
// =====================
if settings.mailer {
    gen.copy_dir("src/mailers");                    // Mailers directory, copied if enabled
}

// =====================
// Background Processing
// =====================
if background {
    gen.copy_template("src/workers/mod.rs.t");       // Workers directory
    gen.copy_dir("tests/workers");                   // Workers test directory
    gen.copy_file("src/workers/downloader.rs");
}

// =====================
// Server side
// =====================
if settings.asset.is_server_side {
    gen.copy_dir("assets");                        // Static assets directory
}

// =====================
// Client side
// =====================
if settings.asset.is_client_side {
   gen.copy_dir("frontend");
   gen.create_file("frontend/dist/index.html", "this is a placeholder. please run your frontend build (npm build)");
}
