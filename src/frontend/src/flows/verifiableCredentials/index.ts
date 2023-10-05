import { authenticateBox } from "$src/components/authenticateBox";
import { showSpinner } from "$src/components/spinner";
import { getDapps } from "$src/flows/dappsExplorer/dapps";
import { authnTemplateManage } from "$src/flows/manage";
import { I18n } from "$src/i18n";
import { Connection } from "$src/utils/iiConnection";
import { Principal } from "@dfinity/principal";
import { allow } from "./allow";
import { vcProtocol } from "./postMessageInterface";

const dapps = getDapps();
const someDapp = dapps.find((dapp) => dapp.name === "NNS Dapp")!;

const giveUp = async (message?: string): Promise<never> => {
  console.error("Nope " + message);
  return await new Promise((_) => {
    /* halt */
  });
};

export const vcFlow = async ({ connection }: { connection: Connection }) => {
  const _result = await vcProtocol({
    onProgress: (x) => {
      if (x === "waiting") {
        return showSpinner({
          message: "Waiting for info",
        });
      }

      if (x === "verifying") {
        return showSpinner({
          message: "Verifying",
        });
      }
      x satisfies never;
    },
    verifyCredentials: async ({ request, rpOrigin }) => {
      // Go through the login flow, potentially creating an anchor.
      const { connection: authenticatedConnection } = await authenticateBox({
        connection,
        i18n: new I18n(),
        templates: authnTemplateManage({ dapps }),
      });

      const { hostname: rpHostname } = new URL(rpOrigin);
      const computedP_RP = await authenticatedConnection.getPrincipal({
        hostname: rpHostname,
      });

      const givenP_RP = Principal.fromText(request.params.credentialSubject);
      // TODO: do some proper principal checking
      if (computedP_RP.toString() !== givenP_RP.toString()) {
        return giveUp(
          [
            "bad principals",
            computedP_RP.toString(),
            givenP_RP.toString(),
          ].join(", ")
        );
      }

      const allowed = await allow({
        relying: someDapp,
        provider: someDapp,
      });
      if (allowed === "canceled") {
        return giveUp("canceled");
      }
      allowed satisfies "allowed";

      return {
        verifiablePresentation: "hello",
      };
    },
  });
};
