import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Ready from "./ui/Ready";
import Welcome from "./ui/Welcome";
import "./index.css";

const PAGES: Record<string, () => React.JSX.Element | null> = {
  "#welcome": Welcome,
  "#ready": Ready,
};
const Page = PAGES[window.location.hash] ?? App;

const root = document.getElementById("root");

if (root) {
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      <Page />
    </React.StrictMode>,
  );
}
