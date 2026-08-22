import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);
root.render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

// 首帧渲染完成后淡出全屏加载动画（防止白屏）
requestAnimationFrame(() => {
  const el = document.getElementById("boot-loading");
  if (el) {
    el.classList.add("fade-out");
    window.setTimeout(() => el.remove(), 300);
  }
});
