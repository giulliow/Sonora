import { GLOSSARY } from "../texts";

/** "O que é cada coisa": as palavras de áudio explicadas sem jargão. */
export function Glossary() {
  return (
    <section className="card">
      <header className="card-head">
        <h2>O que é cada coisa</h2>
      </header>
      <dl className="glossary">
        {GLOSSARY.map((g) => (
          <div key={g.term}>
            <dt>{g.term}</dt>
            <dd>{g.text}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}
