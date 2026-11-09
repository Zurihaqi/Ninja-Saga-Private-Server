"""Guarded, headless repairs for Panels.UI_Skillset in the local client copy.

All owned skills remain visible. Missing optional artwork must not freeze the
inventory or prevent removing a skill from the equipped bar.
"""
import re


def _method(source, name, replacement):
    match = re.search(r"      public function " + re.escape(name) + r"\([^\n]*\n      \{", source)
    if not match:
        raise ValueError(f"UI_Skillset method not found: {name}")
    start = source.index("{", match.start())
    depth = 1
    end = start + 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[:match.start()] + replacement.strip("\n") + source[end:]


def _once(source, old, new):
    if old not in source:
        raise ValueError("UI_Skillset source contract changed: " + old[:75])
    return source.replace(old, new, 1)


def apply(source):
    if "finishSkillIcon" in source:
        return source
    source = _once(source, "      public var selectedCategory:Array;", """      public var selectedCategory:Array;
      public var ownedSkills:Array = [];
      public var equippedLoader:BulkLoader = BulkLoader.createUniqueNamedLoader(8);
      public var previewLoader:BulkLoader = BulkLoader.createUniqueNamedLoader(1);""")
    source = source.replace("this.getElementSkillsArray(int(Character.character_element_1))", "this.getElementSkillsArray(0)")
    source = _once(source, '            case 1:\n               this.currentElement = "wind";', '            case 0:\n               this.currentElement = "all";\n               return this.ownedSkills;\n            case 1:\n               this.currentElement = "wind";')
    source = _once(source, '         if(this.currentElement == "wind")', '         if(this.currentElement == "all")\n         {\n            this.selectedCategory = this.ownedSkills;\n         }\n         else if(this.currentElement == "wind")')
    source = _once(source, '         var _loc1_:* = Character.character_skills.split(",");', '         var _loc1_:* = Character.character_skills.split(",").filter(function(item:*, index:int, array:Array):Boolean { return String(item).length > 0; });\n         this.ownedSkills = _loc1_.concat();')
    source = _once(source, '            if(Character["character_element_" + (_loc1_ + 1)] > 0)', '            if(_loc1_ == 0 || Character["character_element_" + (_loc1_ + 1)] > 0)')
    source = _once(source, '               this["element_mc" + _loc1_].gotoAndStop(Character["character_element_" + (_loc1_ + 1)]);', '               this["element_mc" + _loc1_].gotoAndStop(_loc1_ == 0 ? 1 : Character["character_element_" + (_loc1_ + 1)]);')
    source = _once(source, '               NinjaSage.showDynamicTooltip(this["element_mc" + _loc1_].element,this.getElementName(Character["character_element_" + (_loc1_ + 1)]));', '''               NinjaSage.showDynamicTooltip(this["element_mc" + _loc1_].element,_loc1_ == 0 ? "All owned skills" : this.getElementName(Character["character_element_" + (_loc1_ + 1)]));
               if(_loc1_ == 0)
               {
                  var allLabel:TextField = new TextField();
                  allLabel.text = "ALL";
                  allLabel.textColor = 16777215;
                  allLabel.width = 36;
                  allLabel.height = 22;
                  allLabel.mouseEnabled = false;
                  this["element_mc0"].addChild(allLabel);
               }''')
    source = _once(source, 'this.skillPreset[int(this.selectedPreset) - 1].skills.split(",");', 'this.skillPreset[int(this.selectedPreset) - 1].skills.split(",").filter(function(item:*, index:int, array:Array):Boolean { return String(item).length > 0; });')
    source = _method(source, "onItemLoadError", '''
      public function onItemLoadError(param1:ErrorEvent) : *
      {
         param1.currentTarget.removeEventListener(param1.type,arguments.callee);
         param1.currentTarget.removeEventListener(BulkLoader.COMPLETE,this.completeIcon);
         this.finishSkillIcon(null);
      }''')
    source = _method(source, "completeIcon", '''
      public function completeIcon(param1:Event) : *
      {
         param1.currentTarget.removeEventListener(param1.type,arguments.callee);
         param1.currentTarget.removeEventListener(BulkLoader.ERROR,this.onItemLoadError);
         var content:* = param1.target.content;
         var requested:String = String(this.selectedCategory[this.skillIndex]);
         if(content != null)
         {
            content.stopAllMovieClips();
            if(content.hasOwnProperty(requested) && content[requested] != null) { content[requested].gotoAndStop(1); }
         }
         var icon:MovieClip = content != null && content.hasOwnProperty("icon") ? content.icon as MovieClip : null;
         this.finishSkillIcon(icon);
      }

      public function finishSkillIcon(icon:MovieClip) : *
      {
         var id:String = String(this.selectedCategory[this.skillIndex]);
         var info:Object = SkillLibrary.getSkillInfo(id);
         if(icon == null)
         {
            icon = new MovieClip();
            var label:TextField = new TextField();
            label.text = "?";
            label.textColor = 16777215;
            label.width = 30;
            label.height = 30;
            label.mouseEnabled = false;
            icon.addChild(label);
         }
         if(!Character.play_items_animation) { icon.stopAllMovieClips(); }
         this.skillIconMC.push(icon);
         this["item_" + this.skillCount].iconMC.iconHolder.addChild(icon);
         this["item_" + this.skillCount].gotoAndStop(1);
         this["item_" + this.skillCount].visible = true;
         this["item_" + this.skillCount].txt_name.text = info.skill_name == "null" ? id : info.skill_name;
         this["item_" + this.skillCount].txt_level.text = info.skill_level;
         this["item_" + this.skillCount].btn_equip_skill.visible = int(Character.character_lvl) >= int(info.skill_level) && this.equippedSkill.indexOf(id) < 0;
         this["item_" + this.skillCount].txt_equipped.visible = this.equippedSkill.indexOf(id) >= 0;
         this["item_" + this.skillCount].tooltip = info;
         this.eventHandler.addListener(this["item_" + this.skillCount].btn_equip_skill,MouseEvent.CLICK,this.equipSkill);
         this.eventHandler.addListener(this["item_" + this.skillCount],MouseEvent.CLICK,this.showSkillDetail);
         this.eventHandler.addListener(this["item_" + this.skillCount],MouseEvent.ROLL_OVER,this.toolTiponOver);
         this.eventHandler.addListener(this["item_" + this.skillCount],MouseEvent.ROLL_OUT,this.toolTiponOut);
         ++this.skillIndex;
         ++this.skillCount;
         this.loadSwf();
      }''')
    source = _method(source, "loadEquippedSkills", '''
      public function loadEquippedSkills() : *
      {
         if(this.equippedCount < this.equippedSkill.length && this.equippedCount < 8)
         {
            var path:String = "skills/" + this.equippedSkill[this.equippedCount] + ".swf";
            var item:* = this.equippedLoader.add(path);
            item.addEventListener(BulkLoader.COMPLETE,this.completeEquipIcon);
            item.addEventListener(BulkLoader.ERROR,this.onEquipIconError);
            this.equippedLoader.start();
         }
      }''')
    source = _method(source, "completeEquipIcon", '''
      public function completeEquipIcon(param1:Event) : *
      {
         param1.currentTarget.removeEventListener(param1.type,arguments.callee);
         param1.currentTarget.removeEventListener(BulkLoader.ERROR,this.onEquipIconError);
         var content:* = param1.target.content;
         var requested:String = String(this.equippedSkill[this.equippedCount]);
         if(content != null && content.hasOwnProperty(requested) && content[requested] != null) { content[requested].gotoAndStop(1); }
         var icon:MovieClip = content != null && content.hasOwnProperty("icon") ? content.icon as MovieClip : null;
         this.finishEquippedIcon(icon);
      }

      public function onEquipIconError(param1:ErrorEvent) : *
      {
         param1.currentTarget.removeEventListener(param1.type,arguments.callee);
         param1.currentTarget.removeEventListener(BulkLoader.COMPLETE,this.completeEquipIcon);
         this.finishEquippedIcon(null);
      }

      public function finishEquippedIcon(icon:MovieClip) : *
      {
         if(this.equippedCount >= this.equippedSkill.length || this.equippedCount >= 8) { return; }
         GF.removeAllChild(this["skill_" + this.equippedCount].iconHolder);
         if(icon != null) { this["skill_" + this.equippedCount].iconHolder.addChild(icon); }
         this["skill_" + this.equippedCount].tooltip = SkillLibrary.getSkillInfo(this.equippedSkill[this.equippedCount]);
         this.eventHandler.addListener(this["btn_close_slot" + this.equippedCount],MouseEvent.CLICK,this.removeEquippedSkill);
         this.eventHandler.addListener(this["skill_" + this.equippedCount],MouseEvent.MOUSE_OVER,this.toolTiponOver);
         this.eventHandler.addListener(this["skill_" + this.equippedCount],MouseEvent.MOUSE_OUT,this.toolTiponOut);
         this["skill_" + this.equippedCount].selected = false;
         this.eventHandler.addListener(this["skill_" + this.equippedCount],MouseEvent.CLICK,this.selectSwitchingSkill);
         ++this.equippedCount;
         this.loadEquippedSkills();
      }''')
    source = _once(source, '      public function resetIconHolder() : *\n      {', '      public function resetIconHolder() : *\n      {\n         this.loaderSwf.removeAll();')
    source = _once(source, '      public function resetEquippedIconHolder() : *\n      {', '      public function resetEquippedIconHolder() : *\n      {\n         this.equippedLoader.removeAll();\n         this.equippedCount = 0;')
    source = _once(source, '         this.skillCount = 0;\n         this.equippedCount = 0;', '         this.skillCount = 0;')
    # Prevent old preview requests from sharing the list's counters and callbacks.
    start = source.index('      public function loadSkillAndPreview(')
    end = source.index('      public function completePreview(', start)
    preview = source[start:end].replace('this.loaderSwf', 'this.previewLoader')
    preview = preview.replace('_loc3_.addEventListener(BulkLoader.COMPLETE,this.completePreview);', '_loc3_.addEventListener(BulkLoader.COMPLETE,this.completePreview);\n         _loc3_.addEventListener(BulkLoader.ERROR,this.onPreviewLoadError);')
    source = source[:start] + preview + source[end:]
    source = _once(source, '      public function completePreview(param1:Event) : void\n      {', '''      public function onPreviewLoadError(param1:ErrorEvent) : void
      {
         param1.currentTarget.removeEventListener(BulkLoader.ERROR,this.onPreviewLoadError);
         param1.currentTarget.removeEventListener(BulkLoader.COMPLETE,this.completePreview);
         this.main.loading(false);
         this.main.showMessage("Preview is unavailable for this skill.");
      }

      public function completePreview(param1:Event) : void
      {
         param1.currentTarget.removeEventListener(BulkLoader.ERROR,this.onPreviewLoadError);
         if(!param1.target.content.hasOwnProperty(this.selectedSkill) || param1.target.content[this.selectedSkill] == null)
         {
            this.main.loading(false);
            this.main.showMessage("Preview is unavailable for this skill.");
            return;
         }''')
    source = _once(source, '         this.eventHandler.removeAllEventListeners();', '         this.loaderSwf.removeAll();\n         this.equippedLoader.removeAll();\n         this.previewLoader.removeAll();\n         this.eventHandler.removeAllEventListeners();')
    return source


patch_skills = apply
PATCHES = {"Panels.UI_Skillset": apply}
VERIFICATIONS = {"Panels.UI_Skillset": ["finishSkillIcon", "finishEquippedIcon", "onPreviewLoadError", "All owned skills"]}
