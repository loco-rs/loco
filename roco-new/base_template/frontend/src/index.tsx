import React from "react";
import ReactDOM from "react-dom/client";
import { RocoSplash } from "./RocoSplash";

import "./index.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("No root element found");
}

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <RocoSplash />
  </React.StrictMode>,
);
