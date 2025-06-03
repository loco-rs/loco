to: ".kamal/secrets"
skip_exists: true
message: "Secrets file generated successfully."
---
# Secrets defined here are available for reference under registry/password, env/secret, builder/secrets,
# and accessories/*/env/secret in config/deploy.yml. All secrets should be pulled from either
# password manager, ENV, or a file. DO NOT ENTER RAW CREDENTIALS HERE! This file needs to be safe for git.

# Option 1: Read secrets from the environment
KAMAL_REGISTRY_PASSWORD=$KAMAL_REGISTRY_PASSWORD
{% if postgres -%}
# example: export POSTGRES_PASSWORD="loco"
POSTGRES_PASSWORD=$POSTGRES_PASSWORD
# example: export DATABASE_URL="postgresql://loco:$POSTGRES_PASSWORD@{{pkg_name}}-db:5432/{{pkg_name}}_production"
DATABASE_URL=$DATABASE_URL
{% endif %}
# Option 2: Read secrets via a command
# RAILS_MASTER_KEY=$(cat config/master.key)

# Option 3: Read secrets via kamal secrets helpers
# These will handle logging in and fetching the secrets in as few calls as possible
# There are adapters for 1Password, LastPass + Bitwarden
#
# SECRETS=$(kamal secrets fetch --adapter 1password --account my-account --from MyVault/MyItem KAMAL_REGISTRY_PASSWORD RAILS_MASTER_KEY)
# KAMAL_REGISTRY_PASSWORD=$(kamal secrets extract KAMAL_REGISTRY_PASSWORD $SECRETS)
# RAILS_MASTER_KEY=$(kamal secrets extract RAILS_MASTER_KEY $SECRETS)
