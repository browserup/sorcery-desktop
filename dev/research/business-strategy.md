Our priorities are maybe 80% easy adoption, 20% defensability. Any move we make that adds defensability without really slowing adoption
is a good one.  Our mental model for adoption is below.

@see ./overview.md

@see /Users/ebeland/apps/sorcery-server/dev/remote-links-spec.md for the web server protocol (srcuri.com) as it stands
@see /Users/ebeland/apps/sorcery-desktop/dev/srcuri-protocol-spec.md for the local protocol

Adoption spreads best from small kernerls of use.
* The Sorcery Chrome Extension provides value in-isolation (when Sorcery Desktop is installed) and a base of indidual use and convenience from which the standard can achieve critical mass
  and spread.

Developer Users will
* Be the bottom's up drivers of adoption
* Prefer open source, where possible, but are not too particular about the OSS license type
* Have slightly more friction endorsing for closed-source or proprietary solutions
* Drive enterprises to pay for the service
* Do what is convenient and in their self-interest
* Happily fork, clone, vibe code alternative iplementations when in their self-interest (undesirable)
* Avoid long-term maintenance, or eventually give up where it is needed

Integrators (New Relic, Datadog, Github)
* Will use a "neutral" open standard, but are less likely to use a proprietary system.

Amazon, Google, or the IDE providers may:
* Attempt to strip mine the service, or view "creating a standard" as their domain.
* Are happy to look for opportunities to free-load and produce a competing service (strip mine)

Amazon/Google
* Won't use a GPL/AGPL product, but *may* pay for a license
* May have a motivation to fork if it looks useful, and easy enough, but are slightly less likely to fork a suite

Are slight less likely to strip mine when the product is:
* A suite of products, so it requires multiple initiatives across product managers in different domains
* The product is an accepted standard
* The product involves more on-going and evolving maintenance (for example, aligning the external remote URLs with changes)
* The profit is not at a scale that can move the needle for them
* If they want to use it, when they have a path to use it internally, they

Possible anti strip-mine remedies--make it:
* So protocol "drift" occurs and maintenance would be a factor for a forking developer. For example, publishing the protocol as a permissive library might not be
  a good idea, as it makes it easy to freeload and stay in-sync with the canonical implementation.
* Have a solid web brand solid, short, and usable, so any fork would require. This includes the domain name--so any server fork requires hosting, finding a domain name,
  etc
* Free for individual devs and startups to use the tool. Drive adoption via convenience. Let developers be the bottom-up sales force
* Have convenient, addictive, usage patterns that make it effortless to jump between code, editors/IDEs, merge requests, the terminal, and observability tools
  as possible
* Create the simplest set of rules and mental model as possible for creating, from-scratch, sorcery URLs as possible, with the fewest typed characters

Although the extension is an important part of the ecosystem, my ideal design would be very simple, explainable, and useful *without* the extension being installed.
In other words, the Sorcery Desktop and srcuri.com should be useful even *without* the extension being involved at all.
Developers should be able to follow a simple pattern to make any link share-able.

---
Important questions:

- Is having differences in the protocol (what can be fed to srcuri.com, vs. srcuri://) too much to keep track of?
- Do we want all the file-opens in the world going through our site? (web protocol at srcuri.com vs. srcuri:// links)
- What are the advantages and disadvantages?

	Advantages:  	
		- Is there some sort of business case for this? I'd like to brainstorm on it.
	
	Disadvantages:  
		traffic 
		downtime concerns
		scaling expense

If so, then just focus on the site-based protocol.