import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";

// Both languages carry a prefix (/es/, /en/) and Spanish is the default: a page that is not
// translated yet shows the Spanish one with a notice, and its URL does not change when it is.
// Folder names are the same in both languages (English), because the language picker links a
// page to the same path in the other language.
export default defineConfig({
  site: "https://docs.kosmotheka.com",
  integrations: [
    starlight({
      title: "Kosmotheka",
      defaultLocale: "es",
      locales: {
        es: { label: "Español", lang: "es" },
        en: { label: "English", lang: "en" },
      },
      logo: {
        light: "./src/assets/logotipo-color.svg",
        dark: "./src/assets/logotipo-blanco.svg",
        alt: "Kosmotheka",
        replacesTitle: true,
      },
      favicon: "/favicon.svg",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/kosmotheka/kosmotheka",
        },
      ],
      customCss: ["./src/styles/custom.css"],
      components: {
        Footer: "./src/components/Footer.astro",
        SocialIcons: "./src/components/SocialIcons.astro",
        ThemeSelect: "./src/components/ThemeSelect.astro",
        LanguageSelect: "./src/components/LanguageSelect.astro",
      },
      sidebar: [
        {
          label: "Using the app",
          translations: { es: "Usar la app" },
          items: [{ autogenerate: { directory: "using-the-app" } }],
        },
        {
          label: "Run your server",
          translations: { es: "Montar tu servidor" },
          items: [{ autogenerate: { directory: "run-your-server" } }],
        },
        {
          label: "Contribute",
          translations: { es: "Colaborar" },
          items: [{ autogenerate: { directory: "contribute" } }],
        },
      ],
    }),
  ],
});
