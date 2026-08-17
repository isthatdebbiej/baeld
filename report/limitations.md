# Limitations

- Baeld v0.1 studies CPU governance, not memory reclamation.
- Controlled workloads cannot represent the full behavioral diversity of the web.
- Chromium-native lifecycle freezing is experimental CDP functionality.
- cgroup freezing suspends browser-wide processes and can disrupt real-time work.
- AWS virtual machines can exhibit steal time and noisy-neighbor effects.
- Results apply to the pinned Chromium/Linux configuration and should not be generalized without external reproductions.
- The phase protocol is experimental and is not a stable SDK.
- No claim of production readiness, distributed scheduling, or increased session density is made without direct evidence.

