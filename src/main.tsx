import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { Hud } from "./pages/Hud";
import "./App.css";

const isHud = (window as { __NOVASTRAP_WINDOW__?: string }).__NOVASTRAP_WINDOW__ === "hud" || window.location.hash === "#hud";
document.documentElement.dataset.window = isHud ? "hud" : "main";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{isHud ? <Hud /> : <App />}</React.StrictMode>,
);
