// Orchestration (rôle de main.js dans msg-reader) + raccourcis clavier.
import "./styles.css";
import { FileHandler } from "./FileHandler";
import { MessageHandler } from "./MessageHandler";
import { UIManager } from "./UIManager";

const state = new MessageHandler();
const ui = new UIManager(state);
const files = new FileHandler(state);

document.addEventListener("keydown", (e) => {
  const target = e.target as HTMLElement;
  if (e.ctrlKey || e.metaKey || e.altKey || target.matches("input, textarea")) return;
  switch (e.key) {
    case "ArrowDown":
    case "j":
      state.move(1);
      break;
    case "ArrowUp":
    case "k":
      state.move(-1);
      break;
    case "h":
    case "H":
      state.toggleHeaders(); // FR-035
      break;
    case "p":
    case "P":
      state.togglePin();
      break;
    case "Escape":
      if (state.path.length) state.goToDepth(state.path.length - 1);
      break;
    default:
      return;
  }
  e.preventDefault();
});

void ui.render();
void files.init();
