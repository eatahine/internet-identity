import { authenticateBox } from "$src/components/authenticateBox";
import { showSpinner } from "$src/components/spinner";
import { getDapps } from "$src/flows/dappsExplorer/dapps";
import { authnTemplateManage } from "$src/flows/manage";
import { I18n } from "$src/i18n";
import { Connection } from "$src/utils/iiConnection";
import { vcProtocol } from "./postMessageInterface";
import { prompt } from "./prompt";
import { allow } from "./allow";

const dapps = getDapps();
const someDapp = dapps.find((dapp) => dapp.name === "NNS Dapp")!;

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
    verifyCredentials: async (request) => {
      // Go through the login flow, potentially creating an anchor.
      const {
        userNumber,
        connection: authenticatedConnection,
        newAnchor,
      } = await authenticateBox({
        connection,
        i18n: new I18n(),
        templates: authnTemplateManage({ dapps }),
      });
      const allowed = await allow({
          relying: someDapp,
          provider: someDapp,
      });
      if(allowed === "canceled") {
          console.error("Nope");
            return await new Promise((_) => {
              /* halt */
            });

      }
      allowed satisfies "allowed"

      return {
        id: "1",
        jsonrpc: "2.0",
        result: { verifiablePresentation: "hello" },
      };
    },
  });
};
