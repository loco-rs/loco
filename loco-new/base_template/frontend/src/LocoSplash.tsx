export const LocoSplash = () => {
  return (
    <div>
      <header className="navbar fixed-top">
        <div className="container">
          <a href="https://loco.rs?ref=starter">Loco</a>
          <ul className="navbar-nav ">
            <li className="">
              <a
                className=""
                href="https://github.com/loco-rs/loco?ref=starter"
                target="_blank"
                rel="noreferrer"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="20"
                  height="20"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  className="feather feather-github"
                >
                  <title>Loco GitHub repo</title>
                  <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22" />
                </svg>
              </a>
            </li>
            <li className="">
              <a
                className=""
                href="https://github.com/loco-rs/loco/stargazers?ref=starter"
                target="_blank"
                rel="noreferrer"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="20"
                  height="20"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  className="feather feather-star"
                >
                  <title>Loco GitHub stars</title>
                  <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
                </svg>
              </a>
            </li>
          </ul>
        </div>
      </header>
      <div className="logo">
        <h1>Loco: SaaS application</h1>
        <img src="https://loco.rs/icon.svg" className="logo" alt="Loco logo" />
      </div>
      <footer>
        <ul>
          <li>
            <a
              href="https://loco.rs?ref=starter"
              target="_blank"
              rel="noreferrer"
            >
              Our Documentation
            </a>
          </li>
          <li>
            <a
              href="https://github.com/loco-rs/loco?ref=starter"
              target="_blank"
              rel="noreferrer"
            >
              GitHub
            </a>
          </li>
          <li>
            <a
              href="https://github.com/loco-rs/loco/issues?ref=starter"
              target="_blank"
              rel="noreferrer"
            >
              Found a bug?
            </a>
          </li>
          <li>
            <a
              href="https://github.com/loco-rs/loco/discussions?ref=starter"
              target="_blank"
              rel="noreferrer"
            >
              Needs help?
            </a>
          </li>
        </ul>
      </footer>
    </div>
  );
};
