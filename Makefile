.PHONY: check test headless realm cosmos lab oam reveal

check:
	cargo check --workspace
	cargo test --workspace

test: check

headless:
	cargo run -p qga-app --release -- --headless --frames 8 --scene cosmos

realm:
	cargo run -p qga-app --release -- --scene realm

cosmos:
	cargo run -p qga-app --release -- --scene cosmos

lab:
	cargo run -p qga-app --release -- --scene lab

oam:
	cargo run -p qga-app --release -- --scene oam

reveal:
	cargo run -p qga-app --release -- --scene reveal
