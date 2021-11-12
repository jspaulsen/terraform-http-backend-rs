# Terraform HTTP Backend
Provides an HTTP backend for terraform

## Requirements
### SQLite
Requires version >= 3.35.0

## Configuration
### Server

Server is configured via environment variables:

- `DATABASE_URI` - File path for database; defaults to `/var/lib/terraform-http-backend/state.db`
- `HTTP_PORT` - Port on which server listens; defaults to `8080`
- `HTTP_BIND_ADDRESS` - Address on which server binds; defaults to `0.0.0.0`
- `LOG_LEVEL` - Defines the level at (or above) which messages are logged.

#### Required

The following environment variables must be defined:

- `TF_HTTP_USERNAME` - HTTP username used for basic authentication
- `TF_HTTP_PASSWORD` - HTTP password used for basic authentication


### Terraform

Specify the following backend, populating the relevant fields.  Refer to Terraform documenation [here](https://www.terraform.io/docs/language/settings/backends/http.html) for more configuration options.

```
terraform {
  backend http {
    address = "http://localhost:8080/terraform/${resource_identifier}"
    lock_address = "http://localhost:8080/terraform/${resource_identifier}/lock"
    unlock_address = "http://localhost:8080/terraform/${resource_identifier}/lock"

    lock_method = "POST"
    unlock_method = "DELETE"
  }
}
```

## Build
