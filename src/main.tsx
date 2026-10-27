import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import { I18nProvider } from "./lib/i18n";
import { ThemeProvider } from "./lib/theme";
import { ToastProvider } from "./lib/toast";
import { StoreProvider } from "./lib/store";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <I18nProvider>
      <ThemeProvider>
        <ToastProvider>
          <StoreProvider>
            <App />
          </StoreProvider>
        </ToastProvider>
      </ThemeProvider>
    </I18nProvider>
  </React.StrictMode>,
);
