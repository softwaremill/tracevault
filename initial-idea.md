# TraceVault: Gdy AI pisze kod \- kto pilnuje procesu?

**Dokument wewnętrzny \- SoftwareMill / VirtusLab**   
**Data:** 2026-02-16   
**Status:** Wersja robocza

## 1\. SD(LC) \- wczoraj i jutro

### Jak wyglądał development rok temu

Jeszcze rok temu cykl życia oprogramowania wyglądał znajomo. Developer dostawał ticket z Jiry, tworzył branch, pisał kod, pisał testy, uzupełniał dokumentację, wystawiał pull request. Ktoś z zespołu robił code review, CI uruchamiał pipeline, po akceptacji kod szedł na produkcję.

Developer był autorem każdego kroku. Copilot podpowiadał fragmenty kodu, ale to człowiek decydował, pisał, testował i dokumentował.

### Jak będzie wyglądał za chwilę

W modelu agent-first SDLC developer staje się orkiestratorem. Opisuje intencję \- agent pisze kod, testy i dokumentację. Inny agent robi review. CI weryfikuje wynik. Człowiek akceptuje merge.

To nie jest odległa wizja. Narzędzia takie jak Claude Code czy Cursor już dziś pozwalają agentom na autonomiczną pracę przez dziesiątki minut, z dostępem do terminala, systemu plików i narzędzi CI.

### Kto to robi?

| Krok SDLC | Rok temu | Za chwilę |
| ----- | ----- | ----- |
| Analiza wymagań | Developer czyta ticket | Agent analizuje ticket \+ kontekst repozytorium |
| Pisanie kodu | Developer | Agent (z intencją developera) |
| Pisanie testów | Developer | Agent |
| Code review | Inny developer | Agent-reviewer \+ human approval |
| Dokumentacja | Developer (często pomijana) | Agent (generowana automatycznie z trace'a) |
| CI/CD | Automatyczny pipeline | Automatyczny pipeline \+ walidacja trace'ów |
| Debugowanie | Developer | Agent z dostępem do logów i trace'ów |
| Decyzja o merge | Tech lead / reviewer | Człowiek \- jedyny krok, który pozostaje w 100% ludzki |

Kluczowy wniosek: zmiana nie jest stopniowa. Każdy z tych kroków jest już dziś technicznie możliwy do wykonania przez agenta. Pytanie nie brzmi *czy*, tylko *jak szybko* stanie się to standardem.

Jeden z aspektów związanych z pisaniem oprogramowania wczoraj vs. dziś jest powiązane z pull request’em zaufanie. Jeżeli znamy dobrze osobę która dany PR stworzyła to wiemy czego się możemy spodziewać, nie oznacza to że nie wyłapiemy czegoś w procesie review, aczkolwiek mamy jakieś ogólne mniemanie o kodzie który ostatecznie zaakceptujemy i trafi on na produkcję. Za chwile takiego ‘zaufania’ nie będziemy mieli, praktycznie cały proces pisania nowych funkcjonalności przejmą agenci. 

## 2\. Opus 4.5 → 4.6 \- dowód na tempo zmian

Żeby zrozumieć jak szybko to się dzieje, wystarczy porównać dwa modele wydane w odstępie \~3 miesięcy.

### Porównanie

|  | Opus 4.5 (24 Nov 2025\) | Opus 4.6 (5 Feb 2026\) | Zmiana |
| ----- | ----- | ----- | ----- |
| Okno kontekstowe | 200K tokenów | 200K / **1M tokenów** (beta) | **5x** |
| Maks. output | 64K tokenów | **128K tokenów** | 2x |
| MRCR v2 (retrieval w długim kontekście) | 18.5% | **76.0%** | **\+57.5 pp** |
| ARC-AGI 2 (abstrakcyjne rozumowanie) | 37.6% | **68.8%** | **\+31.2 pp** |
| Terminal-Bench 2.0 (agentic coding) | 59.8% | **65.4%** | \+5.6 pp |
| BrowseComp (agentic web search) | 67.8% | **84.0%** | \+16.2 pp |
| GPQA Diamond (nauka na poziomie PhD) | 87.0% | **91.3%** | \+4.3 pp |
| Humanity's Last Exam | 30.8% | **40.0%** | \+9.2 pp |
| SWE-bench Verified (pisanie kodu) | **80.9%** | 80.8% | \-0.1 pp |
| Tryb myślenia | Manualny (Extended Thinking) | **Automatyczny (Adaptive Thinking)** | \- |
| Agent Teams (wielu agentów) | Nie | **Tak** | \- |

*Źródła: Anthropic (anthropic.com/news/claude-opus-4-6), Vellum (vellum.ai/blog/claude-opus-4-6-benchmarks), dokumentacja API Anthropic.*

### Co to oznacza

Największe skoki nie dotyczą pisania kodu \- SWE-bench stoi w miejscu na \~81%. Ale fundamentalnie zmienia się zdolność modelu do:

* **Pracy z dużym kontekstem.** Retrieval w długim kontekście skoczył z 18.5% do 76% \- model potrafi efektywnie pracować z całą dużą bazą kodu w jednym kontekście.  
* **Abstrakcyjnego rozumowania.** ARC-AGI 2 niemal się podwoił \- agent lepiej rozumie *co* ma zrobić, nie tylko *jak*.  
* **Autonomii.** Agent Teams pozwalają na równoległą pracę wielu agentów z file lockingiem. Adaptive Thinking automatycznie dobiera głębokość rozumowania do złożoności zadania.

Model nie staje się lepszy w pisaniu pojedynczych funkcji. Staje się lepszy w **samodzielnej pracy nad dużymi zadaniami** \- a to jest dokładnie to, co zmienia SD.

Jeśli taki skok następuje w 10-12 tygodni \- jak będzie wyglądać SD za 6 miesięcy?

## 3\. Agent Tracing \- nowy artefakt w SDLC

### Problem: tracimy "dlaczego"

W tradycyjnym SDLC mamy pełną ścieżkę audytu: commity mówią *co* się zmieniło, code review mówi *kto* to zaakceptował, CI logi mówią *czy* testy przeszły. Ale kiedy agent pisze kod, tracimy kluczową informację: **dlaczego** kod został napisany w ten sposób.

Commit mówi co się zmieniło. Ale nie mówi:

* Jakie alternatywy agent rozważył  
* Jaki kontekst miał w momencie pisania  
* Czy uwzględnił kwestie bezpieczeństwa  
* Który model i w jakiej wersji to napisał

### Trace \- zapis procesu myślenia agenta

**Trace** to zapis procesu decyzyjnego agenta \- coś pomiędzy Architecture Decision Record, konwersacją z code review, a logiem buildowym. Ale generowany automatycznie, przy każdej sesji pracy agenta.

Intencja developera → Rozumowanie agenta (TRACE) → Kod \+ Testy \+ Dokumentacja

Trace zawiera:

* Który model i w jakiej wersji został użyty  
* Jaki prompt (intencja) uruchomił pracę  
* Jakie narzędzia zostały wywołane (terminal, pliki, API)  
* Jakie pliki zostały odczytane i zmodyfikowane  
* Jaki był proces rozumowania \- krok po kroku  
* Ile tokenów zostało zużyte na daną operację 

### Standard: Agent Trace Specification

W **styczniu 2026** Cursor opublikował Agent Trace \- otwartą specyfikację dla atrybutowania kodu wygenerowanego przez AI. Format JSON wspierany przez Cognition, Cloudflare, Vercel i Google.

Specyfikacja definiuje *format*. Ale nie istnieje żadna platforma, która:

* Zbiera trace'y z różnych narzędzi  
* Przechowuje je w sposób niezmienny i podpisany kryptograficznie  
* Waliduje je względem polityk compliance *(tu podobnie do Filaru 3 \- Policy As Code w Visdom 2\)*  
* Generuje raporty audytowe

To jest luka, którą może wypełnić TraceVault.

## 4\. TraceVault \- co moglibyśmy zbudować

TraceVault to platforma, która traktuje trace'y agentów jako artefakty compliance \- nie logi debugowe. To wczesna wizja \- szczegóły wykrystalizują się po MVP.

### Audyt i compliance

Kiedy audytor pyta *"jakie kontrole istnieją nad kodem generowanym przez AI?"* \- TraceVault dostarcza odpowiedź: kryptograficznie podpisane, niezmienne zapisy każdej sesji agenta, powiązane z commitami i pull requestami.

Silnik polityk może wymuszać reguły:

* "Każdy AI-wygenerowany kod musi mieć trace"  
* "Trace musi zawierać krok skanowania bezpieczeństwa"  
* "Pliki w katalogu /payments/ wymagają human review"  
* "Blokuj merge gdy trace jest niekompletny"

To odpowiada na wymagania SOX (kontrole wewnętrzne nad zmianami w kodzie), PCI-DSS (bezpieczeństwo danych kart płatniczych) i SR 11-7 (zarządzanie ryzykiem modeli w sektorze finansowym). *(podobno, ale tego nie sprawdzałem)*

### Onboarding i samodokumentujący się kod

Trace'y automatycznie rejestrują *dlaczego* kod został napisany \- działają jak auto-generowane Architecture Decision Records. Nowy developer dołącza do zespołu, czyta trace powiązany z modułem (za pomocą LLM’a, tak żeby trace’y zostały przedstawione w czytelny dla developera/człowieka sposób) i rozumie kontekst decyzji bez pytania kogokolwiek.

Baza kodu dokumentuje się sama \- przez pracę agentów.

To rozwiązuje jeden z najstarszych problemów w software development: dokumentacja, która zawsze jest nieaktualna. Trace jest aktualny z definicji \- powstaje w momencie pisania kodu.

### Metryki i analityka

Dashboard dla CTO i engineering managerów:

* Jaki % kodu jest pisany przez AI (per zespół, per repozytorium)  
* Które modele produkują mniej defektów  
* Kompletność trace'ów w czasie  
* Liczba iteracji agenta per zadanie  
* Trend policy violations

Widoczność tego, jak AI jest naprawdę wykorzystywane w organizacji \- nie anegdoty, a dane.

### Integracje (np. Jira)

Powiązanie trace'ów z ticketami Jira na poziomie issue / story / epic:

* **Zużycie tokenów per ticket** \- ile kosztowało AI wygenerowanie kodu dla danego story  
* **Czas agenta per feature** \- ile sesji, ile iteracji, jaki model  
* **Chargeback dla finance** \- koszt AI per zespół, per projekt  
* **Ścieżka delivery** \- ticket → trace → commit → PR → deploy \- pełna traceability

### MVP: 2 miesiące

*Sam opis MVP to WIP więc nie przywiązujcie do tego zbytniej uwagi*

Pierwszy krok to walidacja pomysłu:

* **TraceVault CLI** \- odczytuje trace'y, waliduje, generuje raport (JSON/SARIF)  
* **GitHub Action** \- uruchamia CLI na PR, komentuje podsumowanie trace'a, blokuje merge przy naruszeniu polityk  
* **Adapter dla Claude Code** \- zbieranie trace'ów z pierwszego wspieranego narzędzia  
* **5 wbudowanych polityk** \- "czy trace istnieje?", "% kodu AI", "atrybucja modelu"

Storage w MVP: git notes (bez oddzielnej bazy danych). Raporty: CLI output (bez dashboardu). Wystarczy, żeby zwalidować pomysł z pierwszymi design partnerami.

## Podsumowanie

1. **SDLC się zmienia** \- agent-first development to nie wizja, to rzeczywistość, która wchodzi do mainstreamu.  
2. **Tempo zmian jest bezprecedensowe** \- skok Opus 4.5 → 4.6 w 10 tygodni pokazuje, że za pół roku możliwości agentów będą fundamentalnie inne.  
3. **Tracimy "dlaczego"** \- gdy agent pisze kod, znika kluczowa informacja o procesie decyzyjnym. Agent Trace Specification definiuje format, ale nie istnieje platforma.  
4. **TraceVault może wypełnić tę lukę** \- audyt, samodokumentacja, metryki i integracje \- dla organizacji, które chcą korzystać z AI odpowiedzialnie.

## 5\. TraceVault a Visdom 2.0 \- komplementarne, nie konkurencyjne

### Czym jest Visdom

Visdom (VirtusLab) to "Autonomous Software Factory" \- platforma, która przygotowuje infrastrukturę deweloperską na pracę z agentami AI. Trzy filary:

| Filar | Co robi | Problem, który rozwiązuje |
| ----- | ----- | ----- |
| **Context Fabric** | MCP serwery karmią agenta wiedzą o architekturze, konwencjach, dokumentacji | "Garbage in, garbage out" \- agent bez kontekstu pisze kod w próżni |
| **Machine-Speed CI** | Bazel \+ Remote Caching \+ Ephemeral Sandboxes | CI zaprojektowane dla ludzi (20 min build) zabija pętlę zwrotną agenta |
| **Auto-Evaluation** | Policy-as-code, auto-merge, deterministic gates | Lawina zmian od agentów \- kto to zreviewuje? |

Visdom odpowiada na pytanie: **jak sprawić, żeby agenci pracowali szybko i poprawnie?**

### Gdzie wchodzi TraceVault

TraceVault odpowiada na inne pytanie: **jak udowodnić, że agenci pracowali poprawnie \- i dlaczego podjęli takie decyzje?**

|  | Visdom | TraceVault |
| ----- | ----- | ----- |
| **Fokus** | Prędkość i jakość | Odpowiedzialność i audyt |
| **Buyer** | CTO / Platform Lead | CCO / Internal Audit / CTO |
| **Wartość** | Agent pisze lepszy kod szybciej | Mamy dowód jak i dlaczego ten kod powstał |
| **Filar** | Context → Execution → Evaluation | Trace → Storage → Policies → Reports |
| **Mierzy** | Czas buildu, iteracje, throughput | % kodu AI, kompletność trace'ów, compliance |

## Weryfikacja danych

| Twierdzenie | Zweryfikowane? | Źródło |
| ----- | ----- | ----- |
| Opus 4.6 wydany 5 Feb 2026 | Tak | anthropic.com/news/claude-opus-4-6 |
| Opus 4.5 wydany 24 Nov 2025 | Tak | anthropic.com/news/claude-opus-4-5 |
| 1M context window (beta) w 4.6 | Tak | Dokumentacja API Anthropic |
| 128K max output w 4.6 | Tak | Dokumentacja API Anthropic |
| SWE-bench: 80.9% vs 80.8% | Tak | vellum.ai/blog/claude-opus-4-6-benchmarks |
| ARC-AGI 2: 37.6% vs 68.8% | Tak | vellum.ai/blog/claude-opus-4-6-benchmarks |
| MRCR v2: 18.5% vs 76% | Tak | anthropic.com/news/claude-opus-4-6 |
| Agent Teams w Claude Code | Tak | anthropic.com/news/claude-opus-4-6 |
| Agent Trace spec \- Cursor, Jan 2026 | Tak | infoq.com/news/2026/02/agent-trace-cursor |
| Agent Trace backed by Cognition, Cloudflare, Vercel, Google | Tak | infoq.com/news/2026/02/agent-trace-cursor |

*Dokument przygotowany na potrzeby wewnętrznej dyskusji SoftwareMill / VirtusLab.*

