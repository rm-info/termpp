# terminal++ — TODO

## v1.1 — Rendre l'app utilisable

- [ ] **Keyboard input vers le PTY** — câbler `iced::keyboard` events → `Emulator::write_input()` pour pouvoir taper dans le terminal
- [ ] **Raccourcis clavier** — lire `config.keybindings` et brancher les actions `SplitPane`, `ClosePane`, `FocusNext`
- [ ] **PTY resize** — détecter les changements de taille de fenêtre/pane et appeler `Pty::resize()` pour mettre à jour les dimensions
- [ ] **Dead pane UI** — afficher un message "Process exited" + raccourci pour relancer quand `PaneStatus::Dead`
- [ ] **Emulateur pour les panes splittés** — déjà partiellement fait, vérifier que le shell démarre bien dans le bon répertoire

## v1.2 — Qualité et confort

- [ ] **Couleurs 256/truecolor** — étendre `apply_sgr` dans `grid.rs` pour les séquences `38;5;n` (256) et `38;2;r;g;b` (truecolor)
- [ ] **OSC 777 robustesse** — gérer les variantes à 3 params (title only) en plus des 4 params (title + body)
- [ ] **Git branch async** — déplacer `detect_git_branch` dans une tâche tokio pour ne pas bloquer le thread UI toutes les 2s
- [ ] **scroll_up O(1)** — remplacer `Vec::remove(0)` par `VecDeque` dans `grid.rs`
- [ ] **Option layout tabs-haut** — ajouter `layout = "tabs"` dans la config TOML
- [ ] **Thèmes additionnels** — déverrouiller la validation de `theme` pour supporter d'autres valeurs

## v2 — Features avancées

- [ ] **CLI/Socket API** — named pipe (Windows) / Unix socket pour piloter l'app depuis des scripts
- [ ] **Port detection** — scanner les ports ouverts et les afficher dans la sidebar
- [ ] **Sauvegarde de session** — sauvegarder/restaurer les workspaces dans des fichiers `.tpp`
- [ ] **Config tabs-haut** — layout alternatif (option brainstormée, reportée en v1.1)
