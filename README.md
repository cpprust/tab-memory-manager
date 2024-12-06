# tab-memory-manager

A memory management strategy more aggressive than Chromium's native memory management mechanism.

Users can configure related options in "~/.config/tab-memory-manager.toml" such as Chromium's maximum memory usage, the idle time threshold for removing tabs, and the memory management strategy to use.

By setting these parameters, users can decide how the program handles memory associated with tabs.

![grafana-dashboard-preview](assets/architecture.webp)

## Usage

- Install browser extension "tab-infos"

  The extension only work with Chromium **dev channel** (Need permission `processes` which only on dev channel).

  Manage extension > check Developer mode > Load unpacked > Select tab-infos

- Build and run "tab-memory-manager"

  It listen on "127.0.0.1:5000" for tab information, which should be given by browser extension.

  ```shell
  cargo run -r
  ```

## Config

Config is "~/.config/tab-memory-manager.toml" on Linux, check [config dir](https://docs.rs/dirs/latest/dirs/fn.config_dir.html).

If it is gone or corrupted, it will be overwrite with default config.

```toml
# Kill the most memory consuming tab in the background with the given strategy
# Options: rss_limit, memory_change_rate, idle_time_limit
kill_tab_strategy = "rss_limit"

# Check interval of choosen strategy
# Range: 0.0 ~ inf
check_interval_secs = 1.0

# Kill the tab if all tabs total resident set size (physical memory usage) hit limit
[strategy.rss_limit]
# Range: 0 ~ 18_446_744_073_709_551_615
max_bytes = 2_000_000_000

# Kill the tab if change rate is too low (not being used)
# 🚧 Not implemented
[strategy.memory_change_rate]
# Range: 0.0 ~ 1.0
min_rate = 0.5

# Kill the tab if idle time is too long
# 🚧 Not implemented
[strategy.idle_time_limit]
# Range: 0.0 ~ inf
max_secs = 30000.0
```

## Grafana dashboard (optional)

The project supports a Grafana visualization dashboard.

After installing Grafana, load the grafana/template.json file to view the memory usage of each Chromium tab, as shown in the example below.

![grafana-dashboard-preview](assets/grafana-dashboard-preview.webp)

- Install grafana

```
sudo pacman -Ss grafana
```

- Import dashboard

- Add data source (json)

  Set url to "127.0.0.1:5000"

- Install grafana plugin

```
sudo grafana cli plugins install marcusolsson-json-datasource
```

- Start grafana

```
sudo systemctl start grafana
```
