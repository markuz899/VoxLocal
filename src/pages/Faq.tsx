import { useSettings } from "../App";
import { FAQ } from "../i18n";
import { PageTitle } from "../ui";

/** Cosa sa fare VoxLocal, nella lingua dell'interfaccia. Domande e risposte sempre aperte. */
export default function Faq() {
  const { settings } = useSettings();
  const faq = settings.ui_language === "it" ? FAQ.it : FAQ.en;
  return (
    <>
      <PageTitle title={faq.title} description={faq.intro} />
      <div className="divide-y divide-line rounded-[10px] border border-line bg-surface">
        {faq.items.map(([q, a]) => (
          <section key={q} className="px-5 py-4">
            <h2 className="text-[15px] font-semibold">{q}</h2>
            <p className="selectable mt-1.5 max-w-[62ch] text-[14px] leading-relaxed">
              {a}
            </p>
          </section>
        ))}
      </div>
    </>
  );
}
