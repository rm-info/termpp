# terminal++ — TODO

## v1.1 — Rendre l'app utilisable

- [ ] **Keyboard input vers le PTY** — câbler `iced::keyboard` events → `Emulator::write_input()` pour pouvoir taper dans le terminal
- [ ] **Raccourcis clavier** — lire `config.keybindings` et brancher les actions `SplitPane`, `ClosePane`, `FocusNext`
- [ ] **PTY resize** — détecter les changements de taille de fenêtre/pane et appeler `Pty::resize()` pour mettre à jour les dimensions
- [ ] **Dead pane UI** — afficher un message "Process exited" + raccourci pour relancer quand `PaneStatus::Dead`
- [ ] **Emulateur pour les panes splittés** — déjà partiellement fait, vérifier que le shell démarre bien dans le bon répertoire

## v1.2 — Qualité et confort

- [x] **Couleurs 256/truecolor** — étendu dans `apply_sgr` (38;5;n, 48;5;n, 38;2;r;g;b, 48;2;r;g;b)
- [x] **OSC 777 robustesse** — variante 3 params gérée
- [x] **Git branch async** — `detect_git_branch` déplacé dans `tokio::task::spawn_blocking`
- [x] **scroll_up O(1)** — `Vec::remove(0)` remplacé par `VecDeque::pop_front()`
- [ ] **Split-view rendering** — afficher plusieurs panes simultanément (côte à côte ou haut/bas) selon la direction du split stockée dans `Layout`; le PTY resize devra tenir compte de la taille individuelle de chaque pane
- [ ] **Renommer les panes** — double-clic sur le nom dans la sidebar pour éditer
- [ ] **Sélection de pane à la souris** — clic sur une entrée de la sidebar pour activer le pane correspondant
- [ ] **Ouvrir/fermer un pane depuis la sidebar** — bouton "+" pour créer, croix pour fermer
- [x] **Auto-close après exit** — option `auto_close_on_exit` dans config (défaut: false)
- [ ] **Option layout tabs-haut** — ajouter `layout = "tabs"` dans la config TOML
- [x] **Thèmes additionnels** — validation de `theme` supprimée, toutes les valeurs acceptées

## v2 — Features avancées

- [ ] **Déplacer les panes à la souris** — drag & drop dans la sidebar pour réorganiser l'ordre des panes
- [ ] **CLI/Socket API** — named pipe (Windows) / Unix socket pour piloter l'app depuis des scripts
- [ ] **Port detection** — scanner les ports ouverts et les afficher dans la sidebar
- [ ] **Sauvegarde de session** — sauvegarder/restaurer les workspaces dans des fichiers `.tpp`
- [ ] **Config tabs-haut** — layout alternatif (option brainstormée, reportée en v1.1)
