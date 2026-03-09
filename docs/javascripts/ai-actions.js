(function () {
  const scriptElement =
    document.currentScript ||
    Array.from(document.scripts).find((script) =>
      script.src.includes("/javascripts/ai-actions.js")
    );
  const manifestUrl = scriptElement
    ? new URL("../assets/ai/page-manifest.json", scriptElement.src).toString()
    : "assets/ai/page-manifest.json";

  let manifestPromise;

  function loadManifest() {
    if (!manifestPromise) {
      manifestPromise = fetch(manifestUrl, { credentials: "same-origin" }).then(
        async (response) => {
          if (!response.ok) {
            throw new Error(`Failed to load AI page manifest: ${response.status}`);
          }
          return response.json();
        }
      );
    }

    return manifestPromise;
  }

  function normalizeRoute(pathname, siteUrl) {
    let relativePath = decodeURIComponent(pathname);

    try {
      const sitePath = new URL(siteUrl).pathname;
      if (sitePath !== "/" && relativePath.startsWith(sitePath)) {
        relativePath = relativePath.slice(sitePath.length);
      }
    } catch (error) {
      console.warn("Unable to parse site URL for AI actions", error);
    }

    relativePath = relativePath.replace(/^\/+/, "").replace(/\/+$/, "");
    relativePath = relativePath.replace(/\/index\.html$/, "");

    if (relativePath === "index.html") {
      return "";
    }

    if (relativePath.endsWith(".html")) {
      relativePath = relativePath.slice(0, -5);
    }

    return relativePath.replace(/\/+$/, "");
  }

  function resolvePage(manifest) {
    const route = normalizeRoute(window.location.pathname, manifest.site_url);
    return manifest.pages[route] || null;
  }

  function buildPrompt(page, manifest) {
    return [
      `Use this ${manifest.site_name} documentation page as the source of truth.`,
      "",
      `Title: ${page.title}`,
      `Canonical URL: ${page.canonical_url}`,
      `Markdown URL: ${page.markdown_url}`,
      `Docs index: ${manifest.llms_txt_url}`,
      `Full docs export: ${manifest.llms_full_url}`,
      "",
      "Prefer the Markdown URL over rendered HTML when you can fetch links.",
    ].join("\n");
  }

  async function copyText(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      await navigator.clipboard.writeText(text);
      return;
    }

    const buffer = document.createElement("textarea");
    buffer.value = text;
    buffer.setAttribute("readonly", "");
    buffer.style.position = "absolute";
    buffer.style.left = "-9999px";
    document.body.appendChild(buffer);
    buffer.select();
    document.execCommand("copy");
    buffer.remove();
  }

  function buildAssistantUrl(target, prompt) {
    const encodedPrompt = encodeURIComponent(prompt);

    switch (target) {
      case "claude":
        return `https://claude.ai/new?q=${encodedPrompt}`;
      case "gemini":
        return `https://gemini.google.com/app?q=${encodedPrompt}`;
      case "chatgpt":
        return `https://chatgpt.com/?q=${encodedPrompt}`;
      default:
        return null;
    }
  }

  function setStatus(statusNode, message) {
    statusNode.textContent = message;
  }

  function createButton(label, action, target) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "ai-doc-fab__button";
    button.dataset.aiAction = action;
    button.textContent = label;

    if (target) {
      button.dataset.aiTarget = target;
    }

    return button;
  }

  function buildToolbar(page, manifest) {
    const toolbar = document.createElement("aside");
    toolbar.className = "ai-doc-fab";

    const details = document.createElement("details");
    details.className = "ai-doc-fab__details";

    const summary = document.createElement("summary");
    summary.className = "ai-doc-fab__toggle";
    summary.setAttribute("aria-label", "Open AI actions");
    summary.textContent = "AI";

    const menu = document.createElement("div");
    menu.className = "ai-doc-fab__menu";

    const title = document.createElement("p");
    title.className = "ai-doc-fab__title";
    title.textContent = "AI Actions";

    const actions = document.createElement("div");
    actions.className = "ai-doc-fab__actions";
    actions.append(
      createButton("Copy", "copy-prompt"),
      createButton("Open in Claude", "open-assistant", "claude"),
      createButton("Open in Gemini", "open-assistant", "gemini"),
      createButton("Open in ChatGPT", "open-assistant", "chatgpt"),
      createButton("Copy Markdown", "copy-markdown")
    );

    const status = document.createElement("p");
    status.className = "ai-doc-fab__status";
    status.setAttribute("aria-live", "polite");

    const exports = document.createElement("div");
    exports.className = "ai-doc-fab__links";

    const llmsLink = document.createElement("a");
    llmsLink.href = manifest.llms_txt_url;
    llmsLink.textContent = "llms.txt";

    const llmsFullLink = document.createElement("a");
    llmsFullLink.href = manifest.llms_full_url;
    llmsFullLink.textContent = "llms-full.txt";

    exports.append(llmsLink, llmsFullLink);
    menu.append(title, actions, exports, status);
    details.append(summary, menu);
    toolbar.append(details);

    toolbar.addEventListener("click", async (event) => {
      const button = event.target.closest("[data-ai-action]");
      if (!button) {
        return;
      }

      const prompt = buildPrompt(page, manifest);

      try {
        if (button.dataset.aiAction === "copy-prompt") {
          await copyText(prompt);
          setStatus(status, "Copied a page-aware prompt to the clipboard.");
          return;
        }

        if (button.dataset.aiAction === "copy-markdown") {
          await copyText(page.markdown);
          setStatus(status, "Copied the source Markdown to the clipboard.");
          return;
        }

        if (button.dataset.aiAction === "open-assistant") {
          await copyText(prompt);
          const assistantUrl = buildAssistantUrl(button.dataset.aiTarget, prompt);
          if (assistantUrl) {
            window.open(assistantUrl, "_blank", "noopener,noreferrer");
          }
          setStatus(
            status,
            `Copied the prompt and opened ${button.dataset.aiTarget}.`
          );
        }
      } catch (error) {
        console.error("AI docs action failed", error);
        setStatus(status, "The action failed. Try again or copy the page URL manually.");
      }
    });

    return toolbar;
  }

  async function renderToolbar(root) {
    const container = root.querySelector(".md-content__inner");
    if (!container || container.querySelector(".ai-doc-fab")) {
      return;
    }

    try {
      const manifest = await loadManifest();
      const page = resolvePage(manifest);

      if (!page) {
        return;
      }

      const toolbar = buildToolbar(page, manifest);
      container.append(toolbar);
    } catch (error) {
      console.error("Unable to render AI docs toolbar", error);
    }
  }

  function boot() {
    if (typeof document$ !== "undefined" && document$.subscribe) {
      document$.subscribe((root) => {
        renderToolbar(root);
      });
      return;
    }

    document.addEventListener(
      "DOMContentLoaded",
      () => {
        renderToolbar(document);
      },
      { once: true }
    );
  }

  boot();
})();
