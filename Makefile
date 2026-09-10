.PHONY: check test headless stills realm cosmos lab oam reveal

check:
	cargo check --workspace
	cargo test --workspace

test: check

headless:
	cargo run -p qga-app --release -- --headless --frames 8 --scene cosmos --profile tiny

stills:
	mkdir -p captures
	cargo run -p qga-app --release -- --headless --frames 8 --scene lab --profile tiny --dump-png captures/lab-tiny.png
	cargo run -p qga-app --release -- --headless --frames 8 --scene realm --profile tiny --dump-png captures/realm-tiny.png
	cargo run -p qga-app --release -- --headless --frames 8 --scene cosmos --profile tiny --integrator verlet --diag --dump-png captures/cosmos-tiny.png
	cargo run -p qga-app --release -- --headless --frames 8 --scene oam --profile tiny --dump-png captures/oam-tiny.png
	cargo run -p qga-app --release -- --headless --frames 8 --scene reveal --profile tiny --dump-png captures/reveal-tiny.png

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
