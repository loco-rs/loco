// =====================
// Base Files
// =====================
// Copy core project structure files and directories that are fundamental
// to the Rust environment, GitHub actions, and formatting settings.

gen.copy_dirs([".cargo", ".github"]);
gen.copy_files([".gitignore", ".rustfmt.toml", "README.md"]);

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
gen.copy_template_dir("src/bin");                   // Copies binary directory with templates


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
   gen.copy_template_dir("migration");                       // Database migrations directory
   gen.copy_dir("src/models");                      // Models directory, copied if background enabled
   gen.copy_file("src/tasks/seed.rs");              // Task to seed database
   gen.copy_dir("src/fixtures");                    // Database fixtures directory
   gen.copy_template("examples/playground.rs.t");   // Example playground template with DB setup

   // Test modules related to database models
   gen.copy_file("tests/models/mod.rs");            // Models tests root
   gen.copy_dir("tests/models/snapshots");          // Test snapshots for models
   gen.copy_template("tests/models/users.rs.t");    // User model test template
   gen.copy_template("tests/tasks/seed.rs.t");      // Seed tasks test template
   gen.copy_template("tests/requests/prepare_data.rs.t"); // Data preparation template
}

// =====================
// Initializers Support
// =====================
if initializers {
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
// Asset Management
// =====================
// Adds asset directory if assets are configured in the app.

if asset {
    gen.copy_dir("assets");                         // Static assets directory
}


// =====================
// Client side
// =====================

if settings.clientside {
   gen.copy_dir("frontend");
   gen.create_file("frontend/dist/index.html", "this is a placeholder. please run your frontend build (npm build)");
}
