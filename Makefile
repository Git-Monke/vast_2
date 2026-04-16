.PHONY: run-server

run-server:
	cargo run -p server

pi:
	pi --models "openrouter/openai/gpt-oss-120b,openrouter/minimax/minimax-m2.7"
