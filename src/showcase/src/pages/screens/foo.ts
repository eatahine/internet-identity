import { authnPages } from "$src/components/authenticateBox";
import { loadIdentityBackground } from "$src/components/identityCard";
import { authnTemplateAuthorize } from "$src/flows/authorize";
import { I18n } from "$src/utils/i18n";

export const i18n = new I18n("en");
export const authnCnfg = {
  register: () => console.log("Register requested"),
  addDevice: () => console.log("Add device requested"),
  recover: () => console.log("Recover requested"),
  onSubmit: (anchor: bigint) => console.log("Submitting anchor", anchor),
};
export const authzTemplates = authnTemplateAuthorize({
  origin: "https://nowhere.com",
  i18n,
});

export const authz = authnPages(i18n, { ...authnCnfg, ...authzTemplates });

export const identityBackground = loadIdentityBackground();
