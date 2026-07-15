//! Seed data for 8 initial industry profiles.
//!
//! Called via `POST /api/modules/industry-profile/seed`.
//! Creates profiles only if they don't already exist (idempotent by slug).
//!
//! Profiles:
//! 1. HVAC (field_service) — full data
//! 2. Pest Control (field_service) — full data
//! 3. Plumbing (field_service) — starter data
//! 4. Electrical (field_service) — starter data
//! 5. Landscaping (field_service) — starter data
//! 6. Dog Waste Removal (field_service) — starter data
//! 7. Law Office (professional) — starter data
//! 8. Pawn Shop (retail) — starter data

use super::profile::*;

/// Generate all 8 seed profiles.
pub fn seed_profiles() -> Vec<IndustryProfile> {
    vec![
        seed_hvac(),
        seed_pest_control(),
        seed_plumbing(),
        seed_electrical(),
        seed_landscaping(),
        seed_dog_waste(),
        seed_law_office(),
        seed_pawn_shop(),
    ]
}

// ── 1. HVAC (full data) ──────────────────────────────────────────────

fn seed_hvac() -> IndustryProfile {
    IndustryProfile {
        id: ulid::Ulid::new().to_string(),
        slug: "hvac".into(),
        name: "HVAC".into(),
        description: "Heating, ventilation, and air conditioning installation, repair, and maintenance services for residential and commercial properties.".into(),
        category: "field_service".into(),
        terminology: vec![
            IndustryTerm {
                term: "SEER Rating".into(),
                definition: "Seasonal Energy Efficiency Ratio — measures cooling efficiency over a typical cooling season. Higher SEER means lower energy costs. Minimum federal standard is 14 SEER (15 in southern states).".into(),
                usage_context: "customer-facing, proposals, equipment specs".into(),
            },
            IndustryTerm {
                term: "AFUE".into(),
                definition: "Annual Fuel Utilization Efficiency — percentage of fuel converted to heat in a furnace. A 96% AFUE furnace converts 96 cents of every dollar spent on fuel into heat.".into(),
                usage_context: "customer-facing, furnace proposals".into(),
            },
            IndustryTerm {
                term: "R-410A".into(),
                definition: "Puron refrigerant — the standard HFC refrigerant that replaced R-22 (Freon). Non-ozone depleting but high GWP. Being phased down under the AIM Act in favor of R-454B.".into(),
                usage_context: "technician notes, compliance documentation".into(),
            },
            IndustryTerm {
                term: "Tonnage".into(),
                definition: "Unit of cooling capacity. One ton equals 12,000 BTU/hour. Residential systems typically range from 1.5 to 5 tons depending on home size and climate zone.".into(),
                usage_context: "customer-facing, load calculations, proposals".into(),
            },
            IndustryTerm {
                term: "Heat Pump".into(),
                definition: "A system that transfers heat between indoor and outdoor air. Provides both heating and cooling from a single unit. Most efficient option in moderate climates.".into(),
                usage_context: "customer-facing, proposals, blog content".into(),
            },
            IndustryTerm {
                term: "Mini-Split".into(),
                definition: "Ductless heating and cooling system with an outdoor compressor and one or more indoor air handlers. Ideal for room additions, older homes without ductwork, or zoned comfort.".into(),
                usage_context: "customer-facing, proposals".into(),
            },
            IndustryTerm {
                term: "Load Calculation".into(),
                definition: "Manual J calculation that determines the correct equipment size for a building based on square footage, insulation, windows, climate zone, and occupancy.".into(),
                usage_context: "technician notes, proposals".into(),
            },
            IndustryTerm {
                term: "Zoning System".into(),
                definition: "A system of dampers and thermostats that divides a home into independently controlled comfort zones, allowing different temperatures in different areas.".into(),
                usage_context: "customer-facing, upsell opportunities".into(),
            },
            IndustryTerm {
                term: "Variable Speed".into(),
                definition: "Refers to a blower motor or compressor that adjusts output incrementally rather than cycling on/off. Improves comfort, efficiency, and humidity control.".into(),
                usage_context: "customer-facing, premium equipment proposals".into(),
            },
            IndustryTerm {
                term: "Refrigerant Charge".into(),
                definition: "The precise amount of refrigerant in a system. Over- or under-charging reduces efficiency and can cause compressor damage. Must be verified during installation and service.".into(),
                usage_context: "technician notes, service reports".into(),
            },
            IndustryTerm {
                term: "Ductwork".into(),
                definition: "The system of metal or flexible channels that distribute conditioned air throughout a building. Leaky or undersized ducts can waste 20-30% of energy.".into(),
                usage_context: "customer-facing, inspection reports".into(),
            },
            IndustryTerm {
                term: "MERV Rating".into(),
                definition: "Minimum Efficiency Reporting Value — rates air filter effectiveness from 1-20. Residential systems typically use MERV 8-13. Higher ratings trap more particles but may restrict airflow.".into(),
                usage_context: "customer-facing, maintenance recommendations".into(),
            },
            IndustryTerm {
                term: "BTU".into(),
                definition: "British Thermal Unit — the amount of heat needed to raise one pound of water by one degree Fahrenheit. Used to measure heating and cooling capacity.".into(),
                usage_context: "customer-facing, equipment specs".into(),
            },
            IndustryTerm {
                term: "Condenser Coil".into(),
                definition: "The outdoor coil in a split system that releases heat absorbed from inside the building. Requires annual cleaning for optimal efficiency.".into(),
                usage_context: "technician notes, maintenance plans".into(),
            },
            IndustryTerm {
                term: "Evaporator Coil".into(),
                definition: "The indoor coil that absorbs heat from the air. Located in the air handler or above the furnace. Prone to freezing if airflow is restricted or refrigerant is low.".into(),
                usage_context: "technician notes, diagnostic reports".into(),
            },
        ],
        compliance_requirements: vec![
            ComplianceReq {
                name: "EPA Section 608 Certification".into(),
                description: "Federal requirement for technicians who purchase, handle, or dispose of refrigerants. Four certification types: Small Appliances, High Pressure, Low Pressure, Universal.".into(),
                required: true,
            },
            ComplianceReq {
                name: "NATE Certification".into(),
                description: "North American Technician Excellence — voluntary industry certification demonstrating competency in specific HVAC specialties. Preferred by major manufacturers.".into(),
                required: false,
            },
            ComplianceReq {
                name: "State Contractor License".into(),
                description: "Most states require HVAC contractors to hold a specialty mechanical or HVAC license, typically requiring exam passage and proof of experience.".into(),
                required: true,
            },
            ComplianceReq {
                name: "AIM Act Refrigerant Phase-Down".into(),
                description: "Federal regulation phasing down HFC refrigerants (including R-410A) by 85% by 2036. New equipment after 2025 must use lower-GWP alternatives like R-454B.".into(),
                required: true,
            },
            ComplianceReq {
                name: "Local Permit Requirements".into(),
                description: "Most jurisdictions require building permits for HVAC installations and major equipment replacements. Includes inspection by local building department.".into(),
                required: true,
            },
        ],
        common_services: vec![
            CommonService {
                name: "AC Installation".into(),
                slug: "ac-installation".into(),
                description: "Complete central air conditioning system installation including equipment, refrigerant lines, electrical connections, and ductwork modifications.".into(),
                price_range: "$3,500 - $7,500".into(),
            },
            CommonService {
                name: "Furnace Installation".into(),
                slug: "furnace-installation".into(),
                description: "Gas or electric furnace replacement including equipment, venting, gas piping, and thermostat setup.".into(),
                price_range: "$2,500 - $6,000".into(),
            },
            CommonService {
                name: "Heat Pump Installation".into(),
                slug: "heat-pump-installation".into(),
                description: "Dual-function heat pump system installation for year-round heating and cooling from a single outdoor unit.".into(),
                price_range: "$4,000 - $8,500".into(),
            },
            CommonService {
                name: "AC Repair".into(),
                slug: "ac-repair".into(),
                description: "Diagnosis and repair of air conditioning malfunctions including refrigerant leaks, compressor failures, and electrical issues.".into(),
                price_range: "$150 - $600".into(),
            },
            CommonService {
                name: "Furnace Repair".into(),
                slug: "furnace-repair".into(),
                description: "Diagnosis and repair of furnace problems including ignition failures, heat exchanger issues, and blower motor replacement.".into(),
                price_range: "$125 - $500".into(),
            },
            CommonService {
                name: "Annual Maintenance / Tune-Up".into(),
                slug: "annual-maintenance".into(),
                description: "Seasonal preventive maintenance including filter change, coil cleaning, refrigerant check, electrical inspection, and safety testing.".into(),
                price_range: "$75 - $200".into(),
            },
            CommonService {
                name: "Duct Cleaning".into(),
                slug: "duct-cleaning".into(),
                description: "Professional cleaning of air ducts using specialized vacuum and agitation equipment to remove dust, debris, and microbial growth.".into(),
                price_range: "$300 - $500".into(),
            },
            CommonService {
                name: "Mini-Split Installation".into(),
                slug: "mini-split-installation".into(),
                description: "Ductless mini-split system installation including outdoor condenser, indoor air handler(s), refrigerant lines, and electrical work.".into(),
                price_range: "$3,000 - $5,000".into(),
            },
        ],
        equipment_categories: vec![
            EquipmentCategory {
                name: "Cooling Systems".into(),
                items: vec![
                    "Central Air Conditioner".into(), "Heat Pump".into(),
                    "Ductless Mini-Split".into(), "Packaged Unit".into(),
                    "Evaporative Cooler".into(),
                ],
            },
            EquipmentCategory {
                name: "Heating Systems".into(),
                items: vec![
                    "Gas Furnace".into(), "Electric Furnace".into(),
                    "Oil Furnace".into(), "Boiler".into(),
                    "Heat Pump".into(), "Radiant Floor".into(),
                ],
            },
            EquipmentCategory {
                name: "Air Quality".into(),
                items: vec![
                    "Air Purifier".into(), "Humidifier".into(),
                    "Dehumidifier".into(), "UV Light System".into(),
                    "ERV/HRV".into(),
                ],
            },
            EquipmentCategory {
                name: "Controls".into(),
                items: vec![
                    "Smart Thermostat".into(), "Programmable Thermostat".into(),
                    "Zoning Dampers".into(), "Zone Control Panel".into(),
                ],
            },
        ],
        material_categories: vec![
            MaterialCategory {
                name: "Refrigerants".into(),
                items: vec![
                    "R-410A (Puron)".into(), "R-32".into(),
                    "R-454B (Opteon XL41)".into(), "R-22 (Freon, phased out)".into(),
                ],
            },
            MaterialCategory {
                name: "Filters".into(),
                items: vec![
                    "1\" Pleated (MERV 8)".into(), "4\" Media Filter (MERV 11)".into(),
                    "HEPA Filter (MERV 17+)".into(), "Washable Electrostatic".into(),
                ],
            },
            MaterialCategory {
                name: "Ductwork".into(),
                items: vec![
                    "Sheet Metal Duct".into(), "Flexible Duct".into(),
                    "Duct Board".into(), "Mastic Sealant".into(),
                    "Foil Tape".into(),
                ],
            },
        ],
        seo_keywords: vec![
            SeoKeyword { keyword: "hvac repair near me".into(), search_volume: Some(74000), difficulty: Some(52), intent: "transactional".into() },
            SeoKeyword { keyword: "ac installation cost".into(), search_volume: Some(33000), difficulty: Some(45), intent: "commercial".into() },
            SeoKeyword { keyword: "furnace not working".into(), search_volume: Some(27000), difficulty: Some(38), intent: "informational".into() },
            SeoKeyword { keyword: "hvac maintenance".into(), search_volume: Some(22000), difficulty: Some(41), intent: "commercial".into() },
            SeoKeyword { keyword: "best hvac company near me".into(), search_volume: Some(18000), difficulty: Some(55), intent: "transactional".into() },
            SeoKeyword { keyword: "heat pump vs furnace".into(), search_volume: Some(14500), difficulty: Some(35), intent: "informational".into() },
            SeoKeyword { keyword: "ductless mini split cost".into(), search_volume: Some(12000), difficulty: Some(42), intent: "commercial".into() },
            SeoKeyword { keyword: "ac not cooling".into(), search_volume: Some(40500), difficulty: Some(33), intent: "informational".into() },
            SeoKeyword { keyword: "hvac tune up".into(), search_volume: Some(18000), difficulty: Some(39), intent: "transactional".into() },
            SeoKeyword { keyword: "emergency hvac repair".into(), search_volume: Some(8100), difficulty: Some(48), intent: "transactional".into() },
            SeoKeyword { keyword: "what seer rating do i need".into(), search_volume: Some(6600), difficulty: Some(25), intent: "informational".into() },
            SeoKeyword { keyword: "hvac financing".into(), search_volume: Some(5400), difficulty: Some(37), intent: "commercial".into() },
            SeoKeyword { keyword: "how long does an ac unit last".into(), search_volume: Some(9900), difficulty: Some(28), intent: "informational".into() },
            SeoKeyword { keyword: "new furnace cost".into(), search_volume: Some(22000), difficulty: Some(43), intent: "commercial".into() },
            SeoKeyword { keyword: "air duct cleaning near me".into(), search_volume: Some(33000), difficulty: Some(50), intent: "transactional".into() },
            SeoKeyword { keyword: "central air conditioning installation".into(), search_volume: Some(6600), difficulty: Some(47), intent: "transactional".into() },
            SeoKeyword { keyword: "hvac contractor near me".into(), search_volume: Some(14800), difficulty: Some(53), intent: "transactional".into() },
            SeoKeyword { keyword: "r-410a refrigerant".into(), search_volume: Some(8100), difficulty: Some(30), intent: "informational".into() },
            SeoKeyword { keyword: "smart thermostat installation".into(), search_volume: Some(5400), difficulty: Some(36), intent: "commercial".into() },
            SeoKeyword { keyword: "hvac system replacement".into(), search_volume: Some(12000), difficulty: Some(46), intent: "commercial".into() },
        ],
        content_guidelines: vec![
            ContentGuideline {
                page_type: "service-page".into(),
                recommended_sections: vec![
                    "Service Overview".into(), "What's Included".into(),
                    "Pricing & Financing".into(), "Why Choose Us".into(),
                    "Equipment Brands We Install".into(), "FAQ".into(),
                    "Service Area".into(), "CTA / Schedule Now".into(),
                ],
                word_count_min: 800,
                word_count_max: 1500,
                tone_notes: "Professional yet approachable. Explain technical concepts in plain language. Emphasize reliability, comfort, and energy savings. Use second person (you/your).".into(),
            },
            ContentGuideline {
                page_type: "homepage".into(),
                recommended_sections: vec![
                    "Hero with CTA".into(), "Services Overview".into(),
                    "Why Choose Us".into(), "Trust Signals (licenses, reviews)".into(),
                    "Service Area Map".into(), "Recent Reviews".into(),
                    "Emergency Service Banner".into(),
                ],
                word_count_min: 500,
                word_count_max: 1000,
                tone_notes: "Confident and trustworthy. Lead with the customer's comfort needs. Highlight 24/7 availability and licensed technicians.".into(),
            },
            ContentGuideline {
                page_type: "about".into(),
                recommended_sections: vec![
                    "Our Story".into(), "Mission & Values".into(),
                    "Team / Technicians".into(), "Certifications & Licenses".into(),
                    "Community Involvement".into(),
                ],
                word_count_min: 400,
                word_count_max: 800,
                tone_notes: "Warm and personal. Share the company's history and commitment to quality. Include specific credentials (EPA certified, NATE certified, years in business).".into(),
            },
            ContentGuideline {
                page_type: "blog-post".into(),
                recommended_sections: vec![
                    "Introduction / Hook".into(), "Main Content (H2 subheadings)".into(),
                    "Expert Tips".into(), "When to Call a Professional".into(),
                    "CTA".into(),
                ],
                word_count_min: 600,
                word_count_max: 1200,
                tone_notes: "Helpful and educational. Position the company as the local expert. Use seasonal topics (e.g., 'Preparing Your Furnace for Winter'). Include internal links to relevant service pages.".into(),
            },
            ContentGuideline {
                page_type: "location-page".into(),
                recommended_sections: vec![
                    "Services in [City]".into(), "Local Info".into(),
                    "Why [City] Residents Trust Us".into(), "Service Area Details".into(),
                    "Reviews from [City] Customers".into(), "Contact / Schedule".into(),
                ],
                word_count_min: 500,
                word_count_max: 900,
                tone_notes: "Locally focused. Reference city-specific details (climate, common home types, local regulations). Avoid generic filler.".into(),
            },
        ],
        seasonal_patterns: vec![
            SeasonalPattern {
                months: vec![6, 7, 8],
                description: "Peak cooling season — AC repairs, installations, and emergency calls surge. Schedule fills 2-3 weeks out.".into(),
                demand_level: "high".into(),
            },
            SeasonalPattern {
                months: vec![11, 12, 1, 2],
                description: "Peak heating season — furnace repairs, no-heat emergencies, and heating system installations dominate.".into(),
                demand_level: "high".into(),
            },
            SeasonalPattern {
                months: vec![3, 4, 5],
                description: "Spring shoulder season — ideal for AC tune-ups, system replacements, and duct cleaning before summer.".into(),
                demand_level: "medium".into(),
            },
            SeasonalPattern {
                months: vec![9, 10],
                description: "Fall shoulder season — furnace tune-ups, heating inspections, and pre-winter system checks.".into(),
                demand_level: "medium".into(),
            },
        ],
        pricing_norms: PricingNorms {
            hourly_rate_range: "$75 - $150".into(),
            service_call_fee_range: "$50 - $100".into(),
            emergency_markup_pct: Some(50.0),
            weekend_markup_pct: Some(25.0),
        },
        customer_pain_points: vec![
            "AC or furnace breaks down on the hottest or coldest day of the year".into(),
            "Hard to tell if a technician is recommending necessary repairs or upselling".into(),
            "Sticker shock on equipment replacement costs".into(),
            "Long wait times to get a technician out during peak season".into(),
            "Not knowing if the company is properly licensed and insured".into(),
            "Unclear pricing — afraid of hidden fees after the work starts".into(),
            "High energy bills but not sure what's causing them".into(),
            "Previous company did a poor installation and now the system underperforms".into(),
        ],
        trust_factors: vec![
            "EPA 608 and NATE certified technicians".into(),
            "Fully licensed, bonded, and insured".into(),
            "Transparent flat-rate pricing — no surprises".into(),
            "24/7 emergency service availability".into(),
            "Manufacturer-authorized dealer (Carrier, Trane, Lennox)".into(),
            "100% satisfaction guarantee".into(),
        ],
        competitor_terms: vec![
            "heating and cooling company".into(),
            "air conditioning service".into(),
            "furnace company near me".into(),
            "HVAC technician".into(),
            "AC company".into(),
        ],
        schema_org_types: vec![
            "HVACBusiness".into(),
            "LocalBusiness".into(),
            "HomeAndConstructionBusiness".into(),
        ],
        active: true,
    }
}

// ── 2. Pest Control (full data) ──────────────────────────────────────

fn seed_pest_control() -> IndustryProfile {
    IndustryProfile {
        id: ulid::Ulid::new().to_string(),
        slug: "pest-control".into(),
        name: "Pest Control".into(),
        description: "Residential and commercial pest management services including inspection, treatment, prevention, and wildlife exclusion.".into(),
        category: "field_service".into(),
        terminology: vec![
            IndustryTerm {
                term: "IPM".into(),
                definition: "Integrated Pest Management — a science-based approach that combines biological, cultural, physical, and chemical methods to minimize pest damage with the least hazard to people, property, and the environment.".into(),
                usage_context: "customer-facing, marketing, compliance documentation".into(),
            },
            IndustryTerm {
                term: "Bait Station".into(),
                definition: "A tamper-resistant container holding rodenticide or insecticide bait. Placed strategically around a property to monitor and reduce pest populations while keeping bait away from children and pets.".into(),
                usage_context: "customer-facing, service reports, treatment plans".into(),
            },
            IndustryTerm {
                term: "Exclusion".into(),
                definition: "Physical pest-proofing of a structure by sealing entry points such as gaps around pipes, cracks in foundations, damaged vents, and gaps under doors. The most effective long-term prevention method.".into(),
                usage_context: "customer-facing, proposals, inspection reports".into(),
            },
            IndustryTerm {
                term: "Residual Treatment".into(),
                definition: "Application of a pesticide that continues to kill or repel pests for weeks or months after application. Applied to baseboards, entry points, and harborage areas.".into(),
                usage_context: "technician notes, service documentation".into(),
            },
            IndustryTerm {
                term: "WDO Inspection".into(),
                definition: "Wood-Destroying Organism inspection — a detailed assessment for termites, carpenter ants, wood-boring beetles, and wood decay fungi. Required for most home sales.".into(),
                usage_context: "customer-facing, real estate documentation".into(),
            },
            IndustryTerm {
                term: "German Cockroach".into(),
                definition: "The most common indoor cockroach species (Blattella germanica). Small, tan-colored, and prolific breeders. Found in kitchens and bathrooms. Resistant to many over-the-counter treatments.".into(),
                usage_context: "customer-facing, treatment plans".into(),
            },
            IndustryTerm {
                term: "Colony Elimination".into(),
                definition: "Complete destruction of a social insect colony (termites, ants) rather than just killing visible workers. Achieved through bait systems or liquid treatments that reach the queen.".into(),
                usage_context: "customer-facing, termite proposals".into(),
            },
            IndustryTerm {
                term: "Harborage".into(),
                definition: "A sheltered area where pests hide, rest, and breed. Examples include wall voids, cluttered storage areas, mulch beds, and underneath appliances.".into(),
                usage_context: "technician notes, inspection reports".into(),
            },
            IndustryTerm {
                term: "Active Ingredient (AI)".into(),
                definition: "The chemical compound in a pesticide formulation responsible for killing or repelling the target pest. Listed on the product label with concentration percentage.".into(),
                usage_context: "technician notes, safety documentation".into(),
            },
            IndustryTerm {
                term: "Monitoring Trap".into(),
                definition: "Non-toxic adhesive or pheromone trap used to detect pest presence and measure population levels. Essential for IPM programs and ongoing monitoring.".into(),
                usage_context: "customer-facing, service reports".into(),
            },
            IndustryTerm {
                term: "Fumigation".into(),
                definition: "Whole-structure treatment using gaseous pesticide (typically sulfuryl fluoride) sealed under a tent. Used for severe drywood termite or bed bug infestations.".into(),
                usage_context: "customer-facing, termite proposals".into(),
            },
            IndustryTerm {
                term: "Perimeter Treatment".into(),
                definition: "Application of a liquid pesticide barrier around the exterior foundation of a building, typically 3 feet up and 3 feet out, to prevent pest entry.".into(),
                usage_context: "customer-facing, service descriptions".into(),
            },
            IndustryTerm {
                term: "Bed Bug Heat Treatment".into(),
                definition: "Non-chemical treatment that raises room temperature to 120-140F for several hours to kill all life stages of bed bugs including eggs.".into(),
                usage_context: "customer-facing, treatment proposals".into(),
            },
            IndustryTerm {
                term: "SDS/MSDS".into(),
                definition: "Safety Data Sheet (formerly Material Safety Data Sheet) — document detailing chemical hazards, handling, storage, and emergency procedures for each pesticide product used.".into(),
                usage_context: "compliance, customer documentation".into(),
            },
            IndustryTerm {
                term: "Re-entry Interval".into(),
                definition: "The minimum time after a pesticide application before people and pets can safely re-enter the treated area. Varies by product, typically 2-4 hours for residential applications.".into(),
                usage_context: "customer-facing, safety documentation".into(),
            },
        ],
        compliance_requirements: vec![
            ComplianceReq {
                name: "State Pesticide Applicator License".into(),
                description: "Required in all states to commercially apply pesticides. Typically requires passing a core exam plus category exams (e.g., General Pest, Termite, Fumigation).".into(),
                required: true,
            },
            ComplianceReq {
                name: "EPA Registration".into(),
                description: "All pesticides used must be registered with the EPA and applied according to label directions. 'The label is the law' — using a product inconsistent with its label is a federal violation.".into(),
                required: true,
            },
            ComplianceReq {
                name: "Business License / Structural Pest Control License".into(),
                description: "State-level business license specifically for pest control operations. Usually requires a qualifying manager with specified years of experience.".into(),
                required: true,
            },
            ComplianceReq {
                name: "Continuing Education (CEU)".into(),
                description: "Most states require annual continuing education credits (typically 4-12 CEU hours) to renew pesticide applicator licenses.".into(),
                required: true,
            },
            ComplianceReq {
                name: "QualityPro Certification".into(),
                description: "National Pest Management Association's voluntary quality standard. Requires background checks, insurance verification, and adherence to best practices.".into(),
                required: false,
            },
        ],
        common_services: vec![
            CommonService {
                name: "General Pest Control".into(),
                slug: "general-pest-control".into(),
                description: "Recurring interior and exterior treatment for common household pests including ants, spiders, roaches, and silverfish.".into(),
                price_range: "$35 - $75/month".into(),
            },
            CommonService {
                name: "Termite Inspection".into(),
                slug: "termite-inspection".into(),
                description: "Thorough inspection for subterranean and drywood termites including moisture assessment and structural damage evaluation.".into(),
                price_range: "$75 - $150".into(),
            },
            CommonService {
                name: "Termite Treatment".into(),
                slug: "termite-treatment".into(),
                description: "Liquid barrier treatment or bait station installation to eliminate and prevent termite colonies from damaging the structure.".into(),
                price_range: "$800 - $2,500".into(),
            },
            CommonService {
                name: "Bed Bug Treatment".into(),
                slug: "bed-bug-treatment".into(),
                description: "Chemical or heat treatment to eliminate bed bug infestations in residential or commercial properties.".into(),
                price_range: "$500 - $2,000".into(),
            },
            CommonService {
                name: "Rodent Control".into(),
                slug: "rodent-control".into(),
                description: "Trapping, baiting, and exclusion services for mice and rats including attic clean-out and entry point sealing.".into(),
                price_range: "$200 - $600".into(),
            },
            CommonService {
                name: "Mosquito Treatment".into(),
                slug: "mosquito-treatment".into(),
                description: "Yard barrier spray and larvicide treatment to reduce mosquito populations during warm months.".into(),
                price_range: "$75 - $150/treatment".into(),
            },
            CommonService {
                name: "Wildlife Exclusion".into(),
                slug: "wildlife-exclusion".into(),
                description: "Humane removal and exclusion of wildlife such as squirrels, raccoons, and bats from attics and crawl spaces.".into(),
                price_range: "$300 - $1,500".into(),
            },
            CommonService {
                name: "Commercial Pest Management".into(),
                slug: "commercial-pest-management".into(),
                description: "Customized IPM program for restaurants, warehouses, healthcare facilities, and other commercial properties.".into(),
                price_range: "$100 - $500/month".into(),
            },
        ],
        equipment_categories: vec![
            EquipmentCategory {
                name: "Application Equipment".into(),
                items: vec![
                    "Backpack Sprayer".into(), "B&G Sprayer".into(),
                    "Power Sprayer".into(), "Dust Applicator".into(),
                    "Bait Gun".into(), "ULV Fogger".into(),
                ],
            },
            EquipmentCategory {
                name: "Monitoring & Trapping".into(),
                items: vec![
                    "Glue Boards".into(), "Pheromone Traps".into(),
                    "Snap Traps".into(), "Live Traps".into(),
                    "Bait Stations".into(), "Moisture Meter".into(),
                ],
            },
            EquipmentCategory {
                name: "Inspection Tools".into(),
                items: vec![
                    "Flashlight (high lumen)".into(), "Telescoping Mirror".into(),
                    "Thermal Camera".into(), "Borescope".into(),
                    "Termite Probing Tool".into(),
                ],
            },
        ],
        material_categories: vec![
            MaterialCategory {
                name: "Insecticides".into(),
                items: vec![
                    "Fipronil".into(), "Bifenthrin".into(),
                    "Imidacloprid".into(), "Indoxacarb".into(),
                    "Chlorfenapyr".into(),
                ],
            },
            MaterialCategory {
                name: "Rodenticides".into(),
                items: vec![
                    "Bromethalin".into(), "Diphacinone".into(),
                    "Cholecalciferol".into(),
                ],
            },
            MaterialCategory {
                name: "Baits & Gels".into(),
                items: vec![
                    "Cockroach Gel Bait".into(), "Ant Gel Bait".into(),
                    "Granular Bait".into(), "Termite Bait Cartridge".into(),
                ],
            },
        ],
        seo_keywords: vec![
            SeoKeyword { keyword: "pest control near me".into(), search_volume: Some(135000), difficulty: Some(58), intent: "transactional".into() },
            SeoKeyword { keyword: "exterminator near me".into(), search_volume: Some(110000), difficulty: Some(55), intent: "transactional".into() },
            SeoKeyword { keyword: "termite treatment cost".into(), search_volume: Some(22000), difficulty: Some(42), intent: "commercial".into() },
            SeoKeyword { keyword: "how to get rid of roaches".into(), search_volume: Some(74000), difficulty: Some(35), intent: "informational".into() },
            SeoKeyword { keyword: "bed bug exterminator".into(), search_volume: Some(33000), difficulty: Some(48), intent: "transactional".into() },
            SeoKeyword { keyword: "rodent control service".into(), search_volume: Some(12000), difficulty: Some(40), intent: "transactional".into() },
            SeoKeyword { keyword: "mosquito control near me".into(), search_volume: Some(18000), difficulty: Some(45), intent: "transactional".into() },
            SeoKeyword { keyword: "pest control cost".into(), search_volume: Some(27000), difficulty: Some(38), intent: "commercial".into() },
            SeoKeyword { keyword: "termite inspection near me".into(), search_volume: Some(22000), difficulty: Some(50), intent: "transactional".into() },
            SeoKeyword { keyword: "wildlife removal near me".into(), search_volume: Some(14500), difficulty: Some(43), intent: "transactional".into() },
            SeoKeyword { keyword: "signs of termites".into(), search_volume: Some(27000), difficulty: Some(30), intent: "informational".into() },
            SeoKeyword { keyword: "ant control service".into(), search_volume: Some(9900), difficulty: Some(37), intent: "transactional".into() },
            SeoKeyword { keyword: "commercial pest control".into(), search_volume: Some(8100), difficulty: Some(44), intent: "commercial".into() },
            SeoKeyword { keyword: "monthly pest control service".into(), search_volume: Some(6600), difficulty: Some(40), intent: "commercial".into() },
            SeoKeyword { keyword: "pest control for new home".into(), search_volume: Some(5400), difficulty: Some(33), intent: "commercial".into() },
            SeoKeyword { keyword: "organic pest control".into(), search_volume: Some(8100), difficulty: Some(36), intent: "informational".into() },
            SeoKeyword { keyword: "spider exterminator".into(), search_volume: Some(12000), difficulty: Some(39), intent: "transactional".into() },
            SeoKeyword { keyword: "rat exterminator near me".into(), search_volume: Some(18000), difficulty: Some(47), intent: "transactional".into() },
            SeoKeyword { keyword: "flea treatment for house".into(), search_volume: Some(14500), difficulty: Some(34), intent: "commercial".into() },
            SeoKeyword { keyword: "wasp nest removal".into(), search_volume: Some(22000), difficulty: Some(32), intent: "transactional".into() },
        ],
        content_guidelines: vec![
            ContentGuideline {
                page_type: "service-page".into(),
                recommended_sections: vec![
                    "Pest Overview".into(), "Signs of Infestation".into(),
                    "Our Treatment Process".into(), "Prevention Tips".into(),
                    "Pricing".into(), "Safety Information".into(),
                    "FAQ".into(), "Schedule Inspection CTA".into(),
                ],
                word_count_min: 800,
                word_count_max: 1500,
                tone_notes: "Reassuring and knowledgeable. Acknowledge the customer's discomfort without being alarmist. Emphasize safety (kids, pets) and effectiveness. Use second person.".into(),
            },
            ContentGuideline {
                page_type: "homepage".into(),
                recommended_sections: vec![
                    "Hero with CTA".into(), "Pests We Treat".into(),
                    "Why Choose Us".into(), "Service Plans".into(),
                    "Free Inspection Offer".into(), "Reviews".into(),
                    "Service Area".into(),
                ],
                word_count_min: 500,
                word_count_max: 1000,
                tone_notes: "Confident and approachable. Lead with the promise of a pest-free home. Highlight safety, guarantees, and response time.".into(),
            },
            ContentGuideline {
                page_type: "blog-post".into(),
                recommended_sections: vec![
                    "Introduction / Hook".into(), "Pest Identification".into(),
                    "DIY vs Professional".into(), "Prevention Steps".into(),
                    "When to Call Us".into(),
                ],
                word_count_min: 600,
                word_count_max: 1200,
                tone_notes: "Educational and helpful. Seasonal topics work well (spring ants, summer mosquitoes, fall rodents). Include images of common pests for identification.".into(),
            },
            ContentGuideline {
                page_type: "about".into(),
                recommended_sections: vec![
                    "Our Story".into(), "Our Approach (IPM)".into(),
                    "Team & Certifications".into(), "Safety Commitment".into(),
                    "Community Involvement".into(),
                ],
                word_count_min: 400,
                word_count_max: 800,
                tone_notes: "Trustworthy and caring. Emphasize family/pet safety and environmental responsibility.".into(),
            },
            ContentGuideline {
                page_type: "location-page".into(),
                recommended_sections: vec![
                    "Pest Control in [City]".into(), "Common Local Pests".into(),
                    "Service Area Details".into(), "Local Reviews".into(),
                    "Free Inspection CTA".into(),
                ],
                word_count_min: 500,
                word_count_max: 900,
                tone_notes: "Locally relevant. Mention region-specific pests (e.g., fire ants in the South, brown recluse in the Midwest). Reference local climate factors.".into(),
            },
        ],
        seasonal_patterns: vec![
            SeasonalPattern {
                months: vec![3, 4, 5],
                description: "Spring surge — ants, termite swarms, and general pest activity increase dramatically as temperatures rise.".into(),
                demand_level: "high".into(),
            },
            SeasonalPattern {
                months: vec![6, 7, 8],
                description: "Peak season — mosquitoes, wasps, fleas, ticks, and roach activity at maximum levels.".into(),
                demand_level: "high".into(),
            },
            SeasonalPattern {
                months: vec![9, 10, 11],
                description: "Fall invasion — rodents, spiders, and stink bugs seek shelter indoors as temperatures drop.".into(),
                demand_level: "medium".into(),
            },
            SeasonalPattern {
                months: vec![12, 1, 2],
                description: "Winter slowdown — mostly rodent work and termite bait monitoring. Good time for commercial accounts.".into(),
                demand_level: "low".into(),
            },
        ],
        pricing_norms: PricingNorms {
            hourly_rate_range: "$50 - $100".into(),
            service_call_fee_range: "$0 - $75".into(),
            emergency_markup_pct: Some(25.0),
            weekend_markup_pct: Some(15.0),
        },
        customer_pain_points: vec![
            "Embarrassment about having pests — worried about what neighbors or guests will think".into(),
            "Concerned about chemicals around children and pets".into(),
            "Frustrated that DIY treatments from the hardware store aren't working".into(),
            "Scared of finding pests in their living space, especially at night".into(),
            "Worried about structural damage from termites or carpenter ants".into(),
            "Unsure if the pest control company will actually solve the problem or just spray and leave".into(),
            "Stressed about recurring infestations despite paying for service".into(),
            "Confused by different treatment options and pricing structures".into(),
        ],
        trust_factors: vec![
            "Licensed and certified applicators in good standing".into(),
            "Family and pet safe treatment methods".into(),
            "Free inspections with no obligation".into(),
            "Satisfaction guarantee with free re-treatments".into(),
            "Eco-friendly and IPM-based approach".into(),
            "Same-day or next-day service available".into(),
        ],
        competitor_terms: vec![
            "exterminator".into(),
            "bug man".into(),
            "pest removal service".into(),
            "pest management company".into(),
            "termite company".into(),
        ],
        schema_org_types: vec![
            "PestControlService".into(),
            "LocalBusiness".into(),
            "HomeAndConstructionBusiness".into(),
        ],
        active: true,
    }
}

// ── 3. Plumbing (starter data) ───────────────────────────────────────

fn seed_plumbing() -> IndustryProfile {
    IndustryProfile {
        id: ulid::Ulid::new().to_string(),
        slug: "plumbing".into(),
        name: "Plumbing".into(),
        description: "Residential and commercial plumbing installation, repair, and drain services.".into(),
        category: "field_service".into(),
        terminology: vec![
            IndustryTerm { term: "Backflow Preventer".into(), definition: "Device installed on water lines to prevent contaminated water from flowing backward into the clean water supply. Required by code on irrigation, fire suppression, and commercial systems.".into(), usage_context: "customer-facing, compliance".into() },
            IndustryTerm { term: "P-Trap".into(), definition: "The curved section of drain pipe beneath sinks that holds water to create a seal preventing sewer gases from entering the home.".into(), usage_context: "customer-facing, service reports".into() },
            IndustryTerm { term: "PEX".into(), definition: "Cross-linked polyethylene flexible piping used for water supply lines. Resistant to freezing damage and easier to install than copper. Color-coded: red for hot, blue for cold.".into(), usage_context: "customer-facing, proposals".into() },
            IndustryTerm { term: "Tankless Water Heater".into(), definition: "On-demand water heating unit that heats water only when needed rather than storing hot water. More energy efficient but higher upfront cost.".into(), usage_context: "customer-facing, proposals".into() },
            IndustryTerm { term: "Sewer Camera Inspection".into(), definition: "Video inspection of sewer lines using a waterproof camera on a flexible cable. Identifies blockages, root intrusion, pipe damage, and bellied sections.".into(), usage_context: "customer-facing, diagnostic reports".into() },
        ],
        compliance_requirements: vec![
            ComplianceReq { name: "State Plumbing License".into(), description: "Required in most states for anyone performing plumbing work for hire. Typically requires journeyman hours and passing a trade exam.".into(), required: true },
            ComplianceReq { name: "Backflow Certification".into(), description: "Separate certification required to test and certify backflow prevention devices. Annual testing mandated by most water utilities.".into(), required: true },
            ComplianceReq { name: "Local Building Permits".into(), description: "Required for new installations, water heater replacements, repipes, and sewer line repairs in most jurisdictions.".into(), required: true },
        ],
        common_services: vec![
            CommonService { name: "Drain Cleaning".into(), slug: "drain-cleaning".into(), description: "Professional clearing of clogged drains using cable machines or hydro-jetting.".into(), price_range: "$100 - $350".into() },
            CommonService { name: "Water Heater Installation".into(), slug: "water-heater-installation".into(), description: "Tank or tankless water heater installation including removal of old unit.".into(), price_range: "$800 - $3,000".into() },
            CommonService { name: "Sewer Line Repair".into(), slug: "sewer-line-repair".into(), description: "Repair or replacement of damaged sewer lines using traditional or trenchless methods.".into(), price_range: "$1,500 - $5,000".into() },
            CommonService { name: "Leak Detection & Repair".into(), slug: "leak-repair".into(), description: "Electronic leak detection and repair of water supply and drain leaks.".into(), price_range: "$150 - $500".into() },
            CommonService { name: "Whole-House Repipe".into(), slug: "repipe".into(), description: "Complete replacement of deteriorated galvanized or polybutylene water supply piping with copper or PEX.".into(), price_range: "$4,000 - $10,000".into() },
        ],
        equipment_categories: vec![
            EquipmentCategory { name: "Drain Equipment".into(), items: vec!["Cable Machine".into(), "Hydro-Jetter".into(), "Sewer Camera".into(), "Locator".into()] },
            EquipmentCategory { name: "Hand Tools".into(), items: vec!["Pipe Wrench".into(), "Basin Wrench".into(), "Tubing Cutter".into(), "PEX Crimp Tool".into(), "Soldering Torch".into()] },
        ],
        material_categories: vec![
            MaterialCategory { name: "Piping".into(), items: vec!["PEX".into(), "Copper".into(), "PVC".into(), "ABS".into(), "Cast Iron".into()] },
            MaterialCategory { name: "Fixtures".into(), items: vec!["Faucets".into(), "Toilets".into(), "Garbage Disposals".into(), "Water Heaters".into()] },
        ],
        seo_keywords: vec![
            SeoKeyword { keyword: "plumber near me".into(), search_volume: Some(246000), difficulty: Some(60), intent: "transactional".into() },
            SeoKeyword { keyword: "emergency plumber".into(), search_volume: Some(49000), difficulty: Some(55), intent: "transactional".into() },
            SeoKeyword { keyword: "drain cleaning service".into(), search_volume: Some(22000), difficulty: Some(45), intent: "transactional".into() },
            SeoKeyword { keyword: "water heater installation cost".into(), search_volume: Some(18000), difficulty: Some(42), intent: "commercial".into() },
            SeoKeyword { keyword: "tankless water heater pros and cons".into(), search_volume: Some(14500), difficulty: Some(30), intent: "informational".into() },
            SeoKeyword { keyword: "sewer line repair cost".into(), search_volume: Some(12000), difficulty: Some(40), intent: "commercial".into() },
            SeoKeyword { keyword: "how to unclog a drain".into(), search_volume: Some(40500), difficulty: Some(28), intent: "informational".into() },
            SeoKeyword { keyword: "plumbing company near me".into(), search_volume: Some(33000), difficulty: Some(55), intent: "transactional".into() },
            SeoKeyword { keyword: "water leak repair".into(), search_volume: Some(14500), difficulty: Some(43), intent: "transactional".into() },
            SeoKeyword { keyword: "repipe cost".into(), search_volume: Some(8100), difficulty: Some(38), intent: "commercial".into() },
        ],
        content_guidelines: vec![
            ContentGuideline { page_type: "service-page".into(), recommended_sections: vec!["Service Overview".into(), "Common Problems We Fix".into(), "Our Process".into(), "Pricing".into(), "FAQ".into(), "CTA".into()], word_count_min: 700, word_count_max: 1400, tone_notes: "Professional and reassuring. Plumbing emergencies are stressful — emphasize fast response and upfront pricing.".into() },
            ContentGuideline { page_type: "homepage".into(), recommended_sections: vec!["Hero".into(), "Services".into(), "Why Choose Us".into(), "Reviews".into(), "Emergency Banner".into()], word_count_min: 400, word_count_max: 900, tone_notes: "Dependable and straightforward. Highlight 24/7 availability and licensed plumbers.".into() },
            ContentGuideline { page_type: "blog-post".into(), recommended_sections: vec!["Introduction".into(), "Problem Explanation".into(), "DIY Steps".into(), "When to Call a Pro".into()], word_count_min: 600, word_count_max: 1200, tone_notes: "Helpful and practical. Plumbing topics are highly searched — focus on common problems.".into() },
        ],
        seasonal_patterns: vec![
            SeasonalPattern { months: vec![11, 12, 1, 2], description: "Frozen and burst pipes, water heater failures spike in cold weather.".into(), demand_level: "high".into() },
            SeasonalPattern { months: vec![6, 7, 8], description: "Sewer line issues from root growth, sprinkler/irrigation work increases.".into(), demand_level: "medium".into() },
        ],
        pricing_norms: PricingNorms {
            hourly_rate_range: "$80 - $175".into(),
            service_call_fee_range: "$50 - $100".into(),
            emergency_markup_pct: Some(50.0),
            weekend_markup_pct: Some(25.0),
        },
        customer_pain_points: vec![
            "Water damage from leaks — worried about mold and structural damage".into(),
            "No hot water — can't shower or do dishes".into(),
            "Sewage backup — health hazard and horrible smell".into(),
            "Fear of being overcharged for a simple repair".into(),
            "Waiting all day for a plumber who doesn't show up on time".into(),
        ],
        trust_factors: vec![
            "Licensed master plumber on staff".into(),
            "Upfront flat-rate pricing before work begins".into(),
            "24/7 emergency service".into(),
            "Written warranty on all repairs".into(),
        ],
        competitor_terms: vec!["plumbing company".into(), "drain service".into(), "plumber".into(), "pipe repair".into()],
        schema_org_types: vec!["Plumber".into(), "LocalBusiness".into(), "HomeAndConstructionBusiness".into()],
        active: true,
    }
}

// ── 4. Electrical (starter data) ─────────────────────────────────────

fn seed_electrical() -> IndustryProfile {
    IndustryProfile {
        id: ulid::Ulid::new().to_string(),
        slug: "electrical".into(),
        name: "Electrical".into(),
        description: "Residential and commercial electrical installation, repair, and safety services.".into(),
        category: "field_service".into(),
        terminology: vec![
            IndustryTerm { term: "GFCI".into(), definition: "Ground Fault Circuit Interrupter — a safety device that shuts off power within milliseconds when it detects current flowing through an unintended path (like water or a person). Required by code in wet areas.".into(), usage_context: "customer-facing, inspection reports".into() },
            IndustryTerm { term: "AFCI".into(), definition: "Arc Fault Circuit Interrupter — breaker that detects dangerous electrical arcs (sparking) and shuts off the circuit to prevent fires. Required by NEC in most living areas.".into(), usage_context: "customer-facing, code compliance".into() },
            IndustryTerm { term: "Panel Upgrade".into(), definition: "Replacing an outdated electrical panel (fuse box or undersized breaker panel) with a modern panel to handle increased electrical load from EV chargers, heat pumps, etc.".into(), usage_context: "customer-facing, proposals".into() },
            IndustryTerm { term: "NEC".into(), definition: "National Electrical Code — the standard for safe electrical installation in the United States. Updated every three years. Local jurisdictions may adopt with amendments.".into(), usage_context: "compliance, technician training".into() },
            IndustryTerm { term: "Load Calculation".into(), definition: "NEC Article 220 calculation that determines the total electrical demand of a building to ensure the service and panel are properly sized.".into(), usage_context: "technician notes, proposals".into() },
        ],
        compliance_requirements: vec![
            ComplianceReq { name: "State Electrical License".into(), description: "Journeyman or master electrician license required in most states. Requires supervised hours (typically 8,000+) and passing a trade exam.".into(), required: true },
            ComplianceReq { name: "NEC Compliance".into(), description: "All electrical work must comply with the National Electrical Code as adopted by the local jurisdiction.".into(), required: true },
            ComplianceReq { name: "Building Permits & Inspections".into(), description: "Permits required for most electrical work beyond basic repairs. Inspected by local building department.".into(), required: true },
        ],
        common_services: vec![
            CommonService { name: "Panel Upgrade".into(), slug: "panel-upgrade".into(), description: "Replace outdated 100-amp or fuse panel with a modern 200-amp breaker panel.".into(), price_range: "$1,500 - $4,000".into() },
            CommonService { name: "EV Charger Installation".into(), slug: "ev-charger-installation".into(), description: "Install Level 2 (240V) electric vehicle charging station in garage or driveway.".into(), price_range: "$800 - $2,000".into() },
            CommonService { name: "Whole House Rewire".into(), slug: "rewire".into(), description: "Complete replacement of outdated knob-and-tube or aluminum wiring with modern copper.".into(), price_range: "$8,000 - $20,000".into() },
            CommonService { name: "Lighting Installation".into(), slug: "lighting-installation".into(), description: "Recessed lighting, fixture upgrades, landscape lighting, and LED conversions.".into(), price_range: "$150 - $500 per fixture".into() },
            CommonService { name: "Generator Installation".into(), slug: "generator-installation".into(), description: "Standby or portable generator installation with transfer switch.".into(), price_range: "$3,000 - $12,000".into() },
        ],
        equipment_categories: vec![
            EquipmentCategory { name: "Panels & Breakers".into(), items: vec!["Main Panel".into(), "Sub-Panel".into(), "AFCI Breaker".into(), "GFCI Breaker".into(), "Transfer Switch".into()] },
        ],
        material_categories: vec![
            MaterialCategory { name: "Wire & Cable".into(), items: vec!["Romex (NM-B)".into(), "THHN".into(), "MC Cable".into(), "UF Cable".into()] },
        ],
        seo_keywords: vec![
            SeoKeyword { keyword: "electrician near me".into(), search_volume: Some(201000), difficulty: Some(58), intent: "transactional".into() },
            SeoKeyword { keyword: "electrical panel upgrade cost".into(), search_volume: Some(22000), difficulty: Some(44), intent: "commercial".into() },
            SeoKeyword { keyword: "ev charger installation".into(), search_volume: Some(27000), difficulty: Some(48), intent: "commercial".into() },
            SeoKeyword { keyword: "emergency electrician".into(), search_volume: Some(18000), difficulty: Some(52), intent: "transactional".into() },
            SeoKeyword { keyword: "whole house rewire cost".into(), search_volume: Some(9900), difficulty: Some(38), intent: "commercial".into() },
            SeoKeyword { keyword: "generator installation near me".into(), search_volume: Some(12000), difficulty: Some(45), intent: "transactional".into() },
            SeoKeyword { keyword: "recessed lighting installation".into(), search_volume: Some(14500), difficulty: Some(40), intent: "commercial".into() },
            SeoKeyword { keyword: "electrical inspection".into(), search_volume: Some(8100), difficulty: Some(35), intent: "commercial".into() },
        ],
        content_guidelines: vec![
            ContentGuideline { page_type: "service-page".into(), recommended_sections: vec!["Service Overview".into(), "Safety Information".into(), "What to Expect".into(), "Pricing".into(), "FAQ".into(), "CTA".into()], word_count_min: 700, word_count_max: 1400, tone_notes: "Safety-first messaging. Emphasize code compliance and the dangers of DIY electrical work.".into() },
            ContentGuideline { page_type: "homepage".into(), recommended_sections: vec!["Hero".into(), "Services".into(), "Why Choose Us".into(), "Certifications".into(), "Reviews".into()], word_count_min: 400, word_count_max: 900, tone_notes: "Professional and safety-conscious. Highlight licensing and insurance prominently.".into() },
        ],
        seasonal_patterns: vec![
            SeasonalPattern { months: vec![5, 6, 7], description: "Surge in outdoor lighting, EV charger, and generator installations before storm season.".into(), demand_level: "high".into() },
            SeasonalPattern { months: vec![11, 12], description: "Holiday lighting installations and generator demand before winter storms.".into(), demand_level: "medium".into() },
        ],
        pricing_norms: PricingNorms {
            hourly_rate_range: "$80 - $200".into(),
            service_call_fee_range: "$50 - $100".into(),
            emergency_markup_pct: Some(50.0),
            weekend_markup_pct: Some(25.0),
        },
        customer_pain_points: vec![
            "Flickering lights or tripping breakers — worried about fire risk".into(),
            "Outdated panel can't handle modern electrical demands".into(),
            "Need EV charger installed but unsure about panel capacity".into(),
            "Concerned about DIY electrical work done by previous homeowner".into(),
        ],
        trust_factors: vec![
            "Licensed master electrician on every job".into(),
            "Background-checked and drug-tested technicians".into(),
            "NEC code-compliant work guaranteed".into(),
            "Upfront pricing with no hidden fees".into(),
        ],
        competitor_terms: vec!["electrical contractor".into(), "electrician".into(), "electrical service".into()],
        schema_org_types: vec!["Electrician".into(), "LocalBusiness".into(), "HomeAndConstructionBusiness".into()],
        active: true,
    }
}

// ── 5. Landscaping (starter data) ────────────────────────────────────

fn seed_landscaping() -> IndustryProfile {
    IndustryProfile {
        id: ulid::Ulid::new().to_string(),
        slug: "landscaping".into(),
        name: "Landscaping".into(),
        description: "Lawn care, landscape design, hardscaping, and outdoor living services for residential and commercial properties.".into(),
        category: "field_service".into(),
        terminology: vec![
            IndustryTerm { term: "Hardscaping".into(), definition: "Non-living landscape elements including patios, retaining walls, walkways, driveways, and outdoor kitchens built from stone, pavers, concrete, or brick.".into(), usage_context: "customer-facing, proposals".into() },
            IndustryTerm { term: "Aeration".into(), definition: "Process of perforating the soil with small holes to allow air, water, and nutrients to reach grass roots. Reduces compaction and promotes healthy turf.".into(), usage_context: "customer-facing, seasonal services".into() },
            IndustryTerm { term: "Overseeding".into(), definition: "Spreading grass seed over an existing lawn to fill in thin or bare spots without tearing up the existing turf. Best done in fall.".into(), usage_context: "customer-facing, seasonal services".into() },
            IndustryTerm { term: "French Drain".into(), definition: "A gravel-filled trench with a perforated pipe that redirects surface and groundwater away from the foundation. Solves drainage and grading issues.".into(), usage_context: "customer-facing, proposals".into() },
            IndustryTerm { term: "Xeriscaping".into(), definition: "Landscape design that reduces or eliminates the need for irrigation by using drought-tolerant plants, mulch, and efficient layout.".into(), usage_context: "customer-facing, design proposals".into() },
        ],
        compliance_requirements: vec![
            ComplianceReq { name: "Pesticide Applicator License".into(), description: "Required for applying lawn care chemicals (herbicides, insecticides, fertilizers) commercially.".into(), required: true },
            ComplianceReq { name: "Landscape Contractor License".into(), description: "Some states require a contractor license for hardscaping and installation work above a dollar threshold.".into(), required: true },
            ComplianceReq { name: "Irrigation License".into(), description: "Some jurisdictions require a separate license for irrigation system installation and backflow prevention.".into(), required: false },
        ],
        common_services: vec![
            CommonService { name: "Weekly Lawn Mowing".into(), slug: "lawn-mowing".into(), description: "Regular lawn mowing, edging, and blowing on a weekly or bi-weekly schedule.".into(), price_range: "$35 - $80/visit".into() },
            CommonService { name: "Landscape Design & Installation".into(), slug: "landscape-design".into(), description: "Custom landscape plans with plant selection, installation, mulching, and grading.".into(), price_range: "$2,000 - $15,000".into() },
            CommonService { name: "Patio / Hardscape Installation".into(), slug: "hardscape-installation".into(), description: "Paver patios, retaining walls, fire pits, and outdoor living spaces.".into(), price_range: "$3,000 - $20,000".into() },
            CommonService { name: "Irrigation System Installation".into(), slug: "irrigation-installation".into(), description: "In-ground sprinkler system with smart controller for efficient watering.".into(), price_range: "$2,500 - $5,000".into() },
            CommonService { name: "Fall/Spring Cleanup".into(), slug: "seasonal-cleanup".into(), description: "Seasonal yard cleanup including leaf removal, bed edging, pruning, and mulching.".into(), price_range: "$200 - $500".into() },
        ],
        equipment_categories: vec![
            EquipmentCategory { name: "Mowing".into(), items: vec!["Zero-Turn Mower".into(), "Walk-Behind Mower".into(), "String Trimmer".into(), "Edger".into(), "Backpack Blower".into()] },
        ],
        material_categories: vec![
            MaterialCategory { name: "Hardscape".into(), items: vec!["Pavers".into(), "Natural Stone".into(), "Retaining Wall Block".into(), "Pea Gravel".into(), "Decomposed Granite".into()] },
        ],
        seo_keywords: vec![
            SeoKeyword { keyword: "landscaping near me".into(), search_volume: Some(110000), difficulty: Some(55), intent: "transactional".into() },
            SeoKeyword { keyword: "lawn care service near me".into(), search_volume: Some(74000), difficulty: Some(52), intent: "transactional".into() },
            SeoKeyword { keyword: "patio installation cost".into(), search_volume: Some(18000), difficulty: Some(42), intent: "commercial".into() },
            SeoKeyword { keyword: "landscape design ideas".into(), search_volume: Some(27000), difficulty: Some(35), intent: "informational".into() },
            SeoKeyword { keyword: "retaining wall cost".into(), search_volume: Some(22000), difficulty: Some(40), intent: "commercial".into() },
        ],
        content_guidelines: vec![
            ContentGuideline { page_type: "service-page".into(), recommended_sections: vec!["Service Overview".into(), "Gallery".into(), "Process".into(), "Pricing".into(), "FAQ".into(), "CTA".into()], word_count_min: 600, word_count_max: 1200, tone_notes: "Visual and aspirational. Landscaping is about transforming outdoor spaces. Use before/after language.".into() },
        ],
        seasonal_patterns: vec![
            SeasonalPattern { months: vec![3, 4, 5, 6], description: "Peak season — mowing contracts start, landscape installations, spring cleanups, planting.".into(), demand_level: "high".into() },
            SeasonalPattern { months: vec![12, 1, 2], description: "Winter slowdown — snow removal in northern regions, minimal maintenance.".into(), demand_level: "low".into() },
        ],
        pricing_norms: PricingNorms {
            hourly_rate_range: "$50 - $100".into(),
            service_call_fee_range: "$0".into(),
            emergency_markup_pct: None,
            weekend_markup_pct: None,
        },
        customer_pain_points: vec![
            "Yard looks terrible compared to neighbors".into(),
            "Don't have time to maintain the lawn".into(),
            "Previous landscaper was unreliable".into(),
            "Want outdoor living space but overwhelmed by options".into(),
        ],
        trust_factors: vec![
            "Licensed and insured".into(),
            "Portfolio of completed projects".into(),
            "Written contracts with clear scope".into(),
            "Consistent weekly schedule".into(),
        ],
        competitor_terms: vec!["lawn service".into(), "yard maintenance".into(), "landscape contractor".into()],
        schema_org_types: vec!["LandscapingService".into(), "LocalBusiness".into()],
        active: true,
    }
}

// ── 6. Dog Waste Removal (starter data) ──────────────────────────────

fn seed_dog_waste() -> IndustryProfile {
    IndustryProfile {
        id: ulid::Ulid::new().to_string(),
        slug: "dog-waste-removal".into(),
        name: "Dog Waste Removal".into(),
        description: "Professional pet waste cleanup services for residential yards, apartment complexes, and commercial properties.".into(),
        category: "field_service".into(),
        terminology: vec![
            IndustryTerm { term: "Pooper Scooper Service".into(), definition: "The common consumer term for professional dog waste removal. Most customers search this phrase rather than the more formal 'pet waste removal'.".into(), usage_context: "customer-facing, SEO content".into() },
            IndustryTerm { term: "Yard Sanitization".into(), definition: "Deep cleaning of outdoor areas including waste removal, enzyme-based deodorizing, and disinfecting to eliminate bacteria, parasites, and odors.".into(), usage_context: "customer-facing, upsell".into() },
            IndustryTerm { term: "Commercial Pet Waste Station".into(), definition: "Wall-mounted or freestanding dispensers with waste bags and attached trash receptacles installed at apartment complexes, HOAs, and dog parks.".into(), usage_context: "commercial proposals".into() },
        ],
        compliance_requirements: vec![
            ComplianceReq { name: "Business License".into(), description: "Standard local business license required in most jurisdictions.".into(), required: true },
            ComplianceReq { name: "Waste Disposal Compliance".into(), description: "Pet waste must be disposed of in accordance with local solid waste regulations. Some areas prohibit disposal in storm drains.".into(), required: true },
        ],
        common_services: vec![
            CommonService { name: "Weekly Yard Cleanup".into(), slug: "weekly-cleanup".into(), description: "Once-weekly visit to thoroughly clean yard of all dog waste.".into(), price_range: "$12 - $20/visit".into() },
            CommonService { name: "Twice-Weekly Cleanup".into(), slug: "twice-weekly-cleanup".into(), description: "Two visits per week for households with multiple dogs.".into(), price_range: "$18 - $30/visit".into() },
            CommonService { name: "One-Time Deep Clean".into(), slug: "deep-clean".into(), description: "Initial deep clean for yards that haven't been maintained, including deodorizing.".into(), price_range: "$75 - $200".into() },
        ],
        equipment_categories: vec![
            EquipmentCategory { name: "Cleanup Tools".into(), items: vec!["Rake & Scooper".into(), "Enzyme Deodorizer".into(), "Commercial Waste Bags".into(), "Sanitizing Spray".into()] },
        ],
        material_categories: vec![],
        seo_keywords: vec![
            SeoKeyword { keyword: "pooper scooper service near me".into(), search_volume: Some(12000), difficulty: Some(35), intent: "transactional".into() },
            SeoKeyword { keyword: "dog waste removal service".into(), search_volume: Some(8100), difficulty: Some(32), intent: "transactional".into() },
            SeoKeyword { keyword: "pet waste cleanup".into(), search_volume: Some(5400), difficulty: Some(28), intent: "transactional".into() },
            SeoKeyword { keyword: "yard poop cleanup service".into(), search_volume: Some(3600), difficulty: Some(25), intent: "transactional".into() },
            SeoKeyword { keyword: "dog poop pickup service cost".into(), search_volume: Some(2900), difficulty: Some(22), intent: "commercial".into() },
        ],
        content_guidelines: vec![
            ContentGuideline { page_type: "homepage".into(), recommended_sections: vec!["Hero with Pricing".into(), "How It Works".into(), "Pricing Plans".into(), "Service Area".into(), "FAQ".into(), "CTA".into()], word_count_min: 400, word_count_max: 800, tone_notes: "Light-hearted and friendly. This is a convenience service — emphasize the joy of a clean yard without the dirty work. Subtle humor is okay.".into() },
        ],
        seasonal_patterns: vec![
            SeasonalPattern { months: vec![3, 4, 5], description: "Spring thaw — massive demand as melting snow reveals months of accumulated waste.".into(), demand_level: "high".into() },
            SeasonalPattern { months: vec![12, 1, 2], description: "Winter slowdown in northern climates. Southern regions remain steady.".into(), demand_level: "low".into() },
        ],
        pricing_norms: PricingNorms {
            hourly_rate_range: "$30 - $50".into(),
            service_call_fee_range: "$0".into(),
            emergency_markup_pct: None,
            weekend_markup_pct: None,
        },
        customer_pain_points: vec![
            "Hate picking up dog poop but love their dogs".into(),
            "Kids can't play in the yard safely".into(),
            "Apartment complex grounds are disgusting".into(),
        ],
        trust_factors: vec![
            "Insured and bonded".into(),
            "Consistent schedule rain or shine".into(),
            "Gate code / key access protocols".into(),
        ],
        competitor_terms: vec!["poop scooping service".into(), "dog poop pickup".into(), "pet waste management".into()],
        schema_org_types: vec!["PetService".into(), "LocalBusiness".into()],
        active: true,
    }
}

// ── 7. Law Office (professional, starter data) ───────────────────────

fn seed_law_office() -> IndustryProfile {
    IndustryProfile {
        id: ulid::Ulid::new().to_string(),
        slug: "law-office".into(),
        name: "Law Office".into(),
        description: "Legal services including consultations, representation, document preparation, and case management for various practice areas.".into(),
        category: "professional".into(),
        terminology: vec![
            IndustryTerm { term: "Retainer".into(), definition: "An advance fee paid by a client to secure a lawyer's services. Held in a trust account and billed against as work is performed.".into(), usage_context: "customer-facing, billing".into() },
            IndustryTerm { term: "Contingency Fee".into(), definition: "Fee arrangement where the attorney's payment is a percentage (typically 33-40%) of the settlement or judgment. Client pays nothing if the case is lost. Common in personal injury.".into(), usage_context: "customer-facing, fee agreements".into() },
            IndustryTerm { term: "Discovery".into(), definition: "Pre-trial phase where both sides exchange relevant information and evidence through interrogatories, depositions, and document requests.".into(), usage_context: "customer-facing, case explanations".into() },
            IndustryTerm { term: "Statute of Limitations".into(), definition: "Legal deadline by which a lawsuit must be filed. Varies by claim type and jurisdiction. Missing this deadline permanently bars the claim.".into(), usage_context: "customer-facing, urgency messaging".into() },
            IndustryTerm { term: "Pro Bono".into(), definition: "Legal work performed free of charge, typically for individuals who cannot afford representation. Latin for 'for the public good'.".into(), usage_context: "marketing, about page".into() },
        ],
        compliance_requirements: vec![
            ComplianceReq { name: "State Bar License".into(), description: "Active license to practice law in the state(s) where the firm operates. Requires passing the bar exam and meeting character & fitness requirements.".into(), required: true },
            ComplianceReq { name: "IOLTA Trust Account".into(), description: "Interest on Lawyers' Trust Accounts — required for holding client funds separate from firm operating accounts.".into(), required: true },
            ComplianceReq { name: "CLE Credits".into(), description: "Continuing Legal Education — annual requirement to maintain bar membership. Varies by state, typically 12-24 hours per year.".into(), required: true },
        ],
        common_services: vec![
            CommonService { name: "Personal Injury".into(), slug: "personal-injury".into(), description: "Representation for accident victims seeking compensation for injuries, medical bills, and lost wages.".into(), price_range: "Contingency: 33-40%".into() },
            CommonService { name: "Family Law".into(), slug: "family-law".into(), description: "Divorce, child custody, support modifications, adoptions, and prenuptial agreements.".into(), price_range: "$200 - $450/hour".into() },
            CommonService { name: "Estate Planning".into(), slug: "estate-planning".into(), description: "Wills, trusts, powers of attorney, and healthcare directives.".into(), price_range: "$500 - $3,000".into() },
            CommonService { name: "Criminal Defense".into(), slug: "criminal-defense".into(), description: "Defense representation for misdemeanor and felony charges.".into(), price_range: "$2,500 - $25,000+".into() },
            CommonService { name: "Business Law".into(), slug: "business-law".into(), description: "Entity formation, contracts, disputes, and regulatory compliance.".into(), price_range: "$200 - $500/hour".into() },
        ],
        equipment_categories: vec![],
        material_categories: vec![],
        seo_keywords: vec![
            SeoKeyword { keyword: "lawyer near me".into(), search_volume: Some(246000), difficulty: Some(62), intent: "transactional".into() },
            SeoKeyword { keyword: "personal injury attorney".into(), search_volume: Some(135000), difficulty: Some(70), intent: "transactional".into() },
            SeoKeyword { keyword: "divorce lawyer near me".into(), search_volume: Some(110000), difficulty: Some(65), intent: "transactional".into() },
            SeoKeyword { keyword: "free consultation lawyer".into(), search_volume: Some(33000), difficulty: Some(55), intent: "transactional".into() },
            SeoKeyword { keyword: "how much does a lawyer cost".into(), search_volume: Some(22000), difficulty: Some(38), intent: "informational".into() },
        ],
        content_guidelines: vec![
            ContentGuideline { page_type: "practice-area-page".into(), recommended_sections: vec!["Overview".into(), "How We Can Help".into(), "Case Results".into(), "Process".into(), "FAQ".into(), "Free Consultation CTA".into()], word_count_min: 800, word_count_max: 1500, tone_notes: "Authoritative and empathetic. Legal situations are stressful — balance expertise with compassion. Avoid legal jargon in consumer-facing content.".into() },
            ContentGuideline { page_type: "homepage".into(), recommended_sections: vec!["Hero".into(), "Practice Areas".into(), "Why Choose Our Firm".into(), "Case Results".into(), "Attorney Profiles".into(), "Reviews".into(), "CTA".into()], word_count_min: 500, word_count_max: 1000, tone_notes: "Confident and trustworthy. Lead with results and experience.".into() },
        ],
        seasonal_patterns: vec![
            SeasonalPattern { months: vec![1, 2], description: "Surge in divorce filings after the holidays ('divorce month'). Personal injury cases from holiday accidents.".into(), demand_level: "high".into() },
            SeasonalPattern { months: vec![6, 7, 8], description: "Increased personal injury from summer activities. Estate planning before vacations.".into(), demand_level: "medium".into() },
        ],
        pricing_norms: PricingNorms {
            hourly_rate_range: "$200 - $500".into(),
            service_call_fee_range: "$0 (free consultations common)".into(),
            emergency_markup_pct: None,
            weekend_markup_pct: None,
        },
        customer_pain_points: vec![
            "Overwhelmed by the legal process and don't know where to start".into(),
            "Afraid of how much a lawyer will cost".into(),
            "Worried about the outcome of their case".into(),
            "Need help urgently but can't find an available attorney".into(),
            "Previous lawyer was unresponsive and didn't keep them informed".into(),
        ],
        trust_factors: vec![
            "Years of experience and case results".into(),
            "Free initial consultation".into(),
            "No fee unless we win (PI cases)".into(),
            "Responsive communication — calls returned same day".into(),
        ],
        competitor_terms: vec!["attorney".into(), "law firm".into(), "legal counsel".into(), "legal services".into()],
        schema_org_types: vec!["Attorney".into(), "LegalService".into(), "ProfessionalService".into()],
        active: true,
    }
}

// ── 8. Pawn Shop (retail, starter data) ──────────────────────────────

fn seed_pawn_shop() -> IndustryProfile {
    IndustryProfile {
        id: ulid::Ulid::new().to_string(),
        slug: "pawn-shop".into(),
        name: "Pawn Shop".into(),
        description: "Pawn lending, buy-sell-trade retail, and collateral-based short-term loans for jewelry, electronics, firearms, tools, and more.".into(),
        category: "retail".into(),
        terminology: vec![
            IndustryTerm { term: "Pawn Loan".into(), definition: "A collateral-based loan where the customer leaves an item of value and receives a cash loan. The customer has a set period (typically 30-90 days) to repay the loan plus interest to reclaim the item.".into(), usage_context: "customer-facing, loan agreements".into() },
            IndustryTerm { term: "Collateral".into(), definition: "The item of value (jewelry, electronics, tools, etc.) that the customer pledges to secure the pawn loan. Held by the pawn shop until the loan is repaid.".into(), usage_context: "customer-facing, loan documents".into() },
            IndustryTerm { term: "Loan-to-Value (LTV)".into(), definition: "The percentage of an item's resale value offered as a loan. Typically 25-60% of estimated resale value, depending on the item category and condition.".into(), usage_context: "internal pricing, staff training".into() },
            IndustryTerm { term: "Default / Forfeiture".into(), definition: "When a customer fails to repay or extend a pawn loan within the grace period. The item becomes property of the pawn shop and can be sold.".into(), usage_context: "customer-facing, loan terms".into() },
            IndustryTerm { term: "Layaway".into(), definition: "Payment plan where a customer makes installment payments toward a purchase. Item is held until fully paid. Common for jewelry and firearms.".into(), usage_context: "customer-facing, retail".into() },
        ],
        compliance_requirements: vec![
            ComplianceReq { name: "Pawnbroker License".into(), description: "State and/or local license required to operate a pawn business. Requirements vary but typically include background checks and bonding.".into(), required: true },
            ComplianceReq { name: "ATF Federal Firearms License".into(), description: "Required if buying/selling firearms. FFL Type 01 (Dealer) or Type 02 (Pawnbroker) with ongoing compliance for 4473 forms and bound book.".into(), required: true },
            ComplianceReq { name: "Police Reporting".into(), description: "Most jurisdictions require pawn shops to report all transactions (including descriptions and serial numbers) to local law enforcement within 24-48 hours.".into(), required: true },
            ComplianceReq { name: "Truth in Lending Act (TILA)".into(), description: "Federal requirement to disclose APR and loan terms to borrowers on all pawn loans.".into(), required: true },
        ],
        common_services: vec![
            CommonService { name: "Pawn Loans".into(), slug: "pawn-loans".into(), description: "Quick cash loans using personal items as collateral. No credit check required.".into(), price_range: "$20 - $5,000+ loans".into() },
            CommonService { name: "Buy / Sell / Trade".into(), slug: "buy-sell-trade".into(), description: "Buy items outright for cash or sell quality pre-owned merchandise at great prices.".into(), price_range: "Varies".into() },
            CommonService { name: "Gold & Jewelry Buying".into(), slug: "gold-buying".into(), description: "Competitive cash offers for gold, silver, platinum, and diamond jewelry based on current spot prices.".into(), price_range: "70-85% of melt value".into() },
            CommonService { name: "Firearms Sales".into(), slug: "firearms".into(), description: "New and pre-owned firearms with required background checks and proper documentation.".into(), price_range: "$100 - $2,000+".into() },
        ],
        equipment_categories: vec![
            EquipmentCategory { name: "Testing Equipment".into(), items: vec!["Gold Acid Test Kit".into(), "Electronic Gold Tester".into(), "Diamond Tester".into(), "Digital Scale".into(), "Loupe (10x)".into()] },
        ],
        material_categories: vec![],
        seo_keywords: vec![
            SeoKeyword { keyword: "pawn shop near me".into(), search_volume: Some(450000), difficulty: Some(55), intent: "transactional".into() },
            SeoKeyword { keyword: "pawn shop that buys gold".into(), search_volume: Some(18000), difficulty: Some(38), intent: "transactional".into() },
            SeoKeyword { keyword: "pawn loan near me".into(), search_volume: Some(12000), difficulty: Some(35), intent: "transactional".into() },
            SeoKeyword { keyword: "sell electronics for cash".into(), search_volume: Some(9900), difficulty: Some(32), intent: "transactional".into() },
            SeoKeyword { keyword: "how do pawn shops work".into(), search_volume: Some(22000), difficulty: Some(25), intent: "informational".into() },
        ],
        content_guidelines: vec![
            ContentGuideline { page_type: "homepage".into(), recommended_sections: vec!["Hero with Loan CTA".into(), "How Pawn Loans Work".into(), "What We Buy".into(), "Featured Inventory".into(), "Reviews".into(), "Location & Hours".into()], word_count_min: 500, word_count_max: 1000, tone_notes: "Welcoming and transparent. Remove stigma around pawn shops. Emphasize fairness, no credit checks, and being a community resource.".into() },
            ContentGuideline { page_type: "service-page".into(), recommended_sections: vec!["How It Works".into(), "What We Accept".into(), "Loan Terms".into(), "FAQ".into(), "CTA".into()], word_count_min: 500, word_count_max: 1000, tone_notes: "Straightforward and honest. Explain the pawn process clearly. Address common misconceptions.".into() },
        ],
        seasonal_patterns: vec![
            SeasonalPattern { months: vec![1, 2], description: "Post-holiday loan demand surges as customers need cash after the holidays.".into(), demand_level: "high".into() },
            SeasonalPattern { months: vec![11, 12], description: "Retail sales peak — customers buying gifts. Loan redemptions increase as people reclaim items for holidays.".into(), demand_level: "high".into() },
        ],
        pricing_norms: PricingNorms {
            hourly_rate_range: "N/A".into(),
            service_call_fee_range: "N/A".into(),
            emergency_markup_pct: None,
            weekend_markup_pct: None,
        },
        customer_pain_points: vec![
            "Need cash fast but have bad credit".into(),
            "Worried about being lowballed on items".into(),
            "Stigma around visiting a pawn shop".into(),
        ],
        trust_factors: vec![
            "Licensed and regulated pawnbroker".into(),
            "Transparent loan terms with no hidden fees".into(),
            "Fair appraisals based on current market values".into(),
            "Clean, well-organized store".into(),
        ],
        competitor_terms: vec!["pawnbroker".into(), "collateral lender".into(), "gold buyer".into(), "used goods store".into()],
        schema_org_types: vec!["PawnShop".into(), "Store".into(), "FinancialService".into()],
        active: true,
    }
}
