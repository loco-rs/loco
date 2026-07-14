to: "nginx/default.conf"
skip_exists: true
message: "Nginx generated successfully."
---
server {
  listen 80;
  server_name ~^(?<subdomain>\w*)\.{{domain}}$;

  location / {
      if ($http_x_subdomain = "") {
          set $http_x_subdomain $subdomain;
      }
      proxy_set_header X-Subdomain $http_x_subdomain;
      proxy_pass http://{{domain}}:{{port}}/;
  }
}

server {
  listen 80;
  server_name {{domain}};

  location / {
      proxy_pass http://{{domain}}:{{port}}/;
  }
}
