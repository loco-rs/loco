# Configuration

Configuration in `loco` lives in `config/` and by default sets up 3 different environments:

```
config/
  development.yaml
  production.yaml
  test.yaml
```

An environment is picked up automatically based on:

- A command line flag: `rr start --environment production`, if not given, fallback to
- `RR_ENV` or `RAILS_ENV` or `NODE_ENV`

When nothing is given, the default value is `development`.
