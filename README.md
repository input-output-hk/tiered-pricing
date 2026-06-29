**Important Disclaimer & Acceptance of Risk**  This is a proof-of-concept implementation that has not undergone security auditing. This code is provided "as is" for research and educational purposes only.  It has not been subjected to a formal security review or audit and may contain vulnerabilities.  **Do not use this code in production systems or any environment where security is critical without conducting your own thorough security assessment.**  By using this code, you acknowledge and accept all associated risks, and our company disclaims any liability for damages or losses.

# tiered-pricing

## What This Repository Is

Tiered Pricing is a research and prototyping project exploring transaction-fee
mechanisms for Cardano's linear-Leios protocol. The current work focuses on
dynamic fees and a paid urgency signal: adjusting fees as demand changes and
giving time-sensitive transactions a protocol-recognised path to faster
inclusion during congestion.

The project combines mechanism design with simulation and experiment tooling.
Candidate designs are evaluated against flat-fee and single-lane controls using
outcomes such as retained transaction value, inclusion rate, latency, fee
stability, throughput, and fairness between urgency classes.

The repository contains:

- [`abstract-sim-hs/`](abstract-sim-hs/) - the Haskell mechanism simulator and
  seeded experiment-sweep runner
- [`abstract-sim-viz/`](abstract-sim-viz/) - a browser-based dashboard for
  inspecting and comparing simulator traces
- [`docs/`](docs/) - research notes, mechanism design, experiment reports, and
  the draft urgency-signalling CPS

This is a proof of concept and a research artifact, not a production-ready
transaction-pricing implementation.

## Getting Started

The primary experiment workflow uses the Haskell simulator. The recommended
development environment is [Nix](https://nixos.org/) with flakes enabled; the
root flake provides GHC 9.10.3, Stack, Haskell Language Server, Python 3, and
pytest.

```sh
git clone https://github.com/input-output-hk/tiered-pricing.git
cd tiered-pricing
nix develop

cd abstract-sim-hs
stack test
```

To reproduce the phase-2 mechanism experiment using its committed design
matrix, seed count, run length, and output location:

```sh
stack run -- sweep config/sweeps/mechanisms.json
```

The sweep writes one JSONL trace per design and seed, plus a combined
`summary.json`, to the output directory declared by the manifest. Experiment
definitions live in
[`config/sweeps/`](abstract-sim-hs/config/sweeps/), while individual mechanism
configurations live in
[`config/variants/`](abstract-sim-hs/config/variants/).

To inspect the experiment traces in the dashboard, leave the Nix shell active
and run:

```sh
cd ../abstract-sim-viz
python3 preprocess.py \
  ../abstract-sim-hs/sweep-results/mechanisms/*.events.jsonl
```

Open `dashboard/index.html` with your browser of choice.

See the component README for the
[`visualisation workflow`](abstract-sim-viz/README.md), and start with the
[`phase-2 mechanism design`](docs/phase-2/mechanism-design.md) for the current
design rationale.

## Contributing

Issues and pull requests are welcome. Before starting a substantial change to
the mechanism design, simulator behaviour, or experiment methodology, please
[open an issue](https://github.com/input-output-hk/tiered-pricing/issues) to
agree on its scope and assumptions.

When submitting a pull request:

- Explain the problem, the proposed approach, and any modelling assumptions.
- Keep experiments reproducible: commit mechanism configurations under
  [`config/variants/`](abstract-sim-hs/config/variants/) and experiment matrices
  under [`config/sweeps/`](abstract-sim-hs/config/sweeps/).
- Add or update tests for behavioural changes, and update the relevant
  documentation when metrics, configuration, or mechanism semantics change.
- Include the commands used to validate the change and, for experimental work,
  a concise summary of the resulting evidence.

Run the checks relevant to the components you changed:

- Haskell simulator: `cd abstract-sim-hs && stack test`
- Visualisation: `cd abstract-sim-viz && python3 -m pytest`

## License

Copyright 2026 Input Output Global


Licensed under the Apache License, Version 2.0 (the "License"). You may not use this repository except in compliance with the License. You may obtain a copy of the License at http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License
