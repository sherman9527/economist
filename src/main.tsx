import React from "react";
import ReactDOM from "react-dom/client";
import "./styles/tokens.css";
import "./styles/layout.css";
import "./styles/components.css";
import { App } from "./App";

const rootNode = document.getElementById("root");
if (rootNode) {
  ReactDOM.createRoot(rootNode).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
}
