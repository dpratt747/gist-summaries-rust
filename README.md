Open WebUI account

**username**: david.pratt@kaluza.com

**password**: password

## Requirements:
- Docker Desktop
  - Enable host-side TCP 


https://docs.docker.com/ai/compose/models-and-compose/
https://docs.docker.com/ai/model-runner/

Running the application:
`docker compose build gist-summary && docker compose run --rm gist-summary`
or
`./start.sh`


Required environment variables:
- `OPENAI_API_KEY`
- `OPENAI_BASE_URL`
- `GITHUB_TOKEN`

---
## Packaging
```bash
cd /Users/david.prattkaluza.com/RustroverProjects/gist-summary
npx --prefix front-end tauri build
```