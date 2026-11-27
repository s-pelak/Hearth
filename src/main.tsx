import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { initPlatform } from "./native/platform";
import "./native/brand.css";
import "./native/platform.css";

initPlatform().then(() => {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});
