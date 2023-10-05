import { mainWindow } from "$src/components/mainWindow";
import { KnownDapp } from "$src/flows/dappsExplorer/dapps";
import { mount, renderPage } from "$src/utils/lit-html";
import { TemplateResult, html } from "lit-html";

/* Anchor construction component (for creating WebAuthn credentials) */

const allowTemplate = ({
  relying,
  provider,
  onAllow,
  onCancel,
  scrollToTop = false,
}: {
  relying: KnownDapp;
  provider: KnownDapp;
  onAllow: () => void;
  onCancel: () => void;
  /* put the page into view */
  scrollToTop?: boolean;
}): TemplateResult => {
  const slot = html`
    <hgroup ${scrollToTop ? mount(() => window.scrollTo(0, 0)) : undefined}>
      <h1 class="t-title t-title--main">Credential Access Request,</h1>
    </hgroup>
    <p class="t-paragraph">
      Allow sharing the following credential issued by
      <strong class="t-strong">${provider.name}</strong> with
      <strong class="t-strong">${relying.name}</strong>?
    </p>

    <div class="c-button-group">
      <button
        data-action="cancel"
        class="c-button c-button--secondary"
        @click="${() => onCancel()}"
      >
        Cancel
      </button>
      <button data-action="allow" class="c-button" @click="${() => onAllow()}">
        Allow
      </button>
    </div>
  `;

  return mainWindow({
    showFooter: false,
    showLogo: false,
    slot,
  });
};

export const allowPage = renderPage(allowTemplate);

// Prompt the user to create a WebAuthn identity
export const allow = ({
  relying,
  provider,
}: {
  relying: KnownDapp;
  provider: KnownDapp;
}): Promise<"allowed" | "canceled"> => {
  return new Promise((resolve) =>
    allowPage({
      relying,
      provider,
      onAllow: () => resolve("allowed"),
      onCancel: () => resolve("canceled"),
      scrollToTop: true,
    })
  );
};
