import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

const bootSplash = document.getElementById("boot-splash");
if (bootSplash) {
  window.requestAnimationFrame(() => {
    bootSplash.classList.add("hidden");
    window.setTimeout(() => {
      bootSplash.remove();
    }, 220);
  });
}
