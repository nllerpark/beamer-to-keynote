on splitText(sourceText, delimiterText)
	set previousDelimiters to AppleScript's text item delimiters
	set AppleScript's text item delimiters to delimiterText
	set textParts to text items of sourceText
	set AppleScript's text item delimiters to previousDelimiters
	return textParts
end splitText

on run argv
	if (count of argv) is not 5 then error "필수 인수가 올바르지 않습니다."

	set outputPath to item 1 of argv
	set deckWidth to (item 2 of argv) as integer
	set deckHeight to (item 3 of argv) as integer
	set manifestPath to item 4 of argv
	set keepOpen to (item 5 of argv) is "true"

	set manifestContent to read (POSIX file manifestPath) as «class utf8»
	set manifestRows to paragraphs of manifestContent
	set keynoteWasRunning to running of application "Keynote"
	set deckDocument to missing value
	set expectedSlides to 0

	try
		tell application "Keynote"
			launch
			repeat 20 times
				if running then exit repeat
				delay 0.25
			end repeat
			set chosenTheme to first theme
			set deckDocument to make new document with properties {document theme:chosenTheme, width:deckWidth, height:deckHeight}
			set sourceLayout to base layout of slide 1 of deckDocument
			set currentSlide to missing value

			repeat with manifestRow in manifestRows
				set rowText to manifestRow as text
				if rowText is not "" then
					set rowFields to my splitText(rowText, tab)
					set rowType to item 1 of rowFields

					if rowType is "SLIDE" then
						set slideIndex to (item 2 of rowFields) as integer
						set expectedSlides to expectedSlides + 1
						if slideIndex is 1 then
							set currentSlide to slide 1 of deckDocument
						else
							tell deckDocument to set currentSlide to make new slide at end of slides with properties {base layout:sourceLayout}
						end if
						tell currentSlide
							set title showing to false
							set body showing to false
							set presenter notes to item 3 of rowFields
						end tell

					else if rowType is "BACKGROUND" then
						set vectorPath to item 3 of rowFields
						tell currentSlide
							set vectorLayer to make new image with properties {file:(POSIX file vectorPath)}
							set position of vectorLayer to {0, 0}
							set width of vectorLayer to deckWidth
							set height of vectorLayer to deckHeight
							set description of vectorLayer to "TeXKey vector graphics"
							set locked of vectorLayer to true
						end tell

					else if rowType is "TEXT" or rowType is "ROTATED_TEXT" then
						if (count of rowFields) < 14 then error "텍스트 행의 필드가 손상되었습니다: " & rowText
						set protectedText to item 3 of rowFields
						if protectedText starts with "@" then
							set itemText to text 2 thru -1 of protectedText
						else
							set itemText to protectedText
						end if
						set itemFont to item 4 of rowFields
						set itemSize to (item 5 of rowFields) as real
						if itemSize ≤ 0 then error "텍스트 크기가 올바르지 않습니다: " & rowText
						set itemRed to (item 6 of rowFields) as integer
						set itemGreen to (item 7 of rowFields) as integer
						set itemBlue to (item 8 of rowFields) as integer
						set itemX to (item 9 of rowFields) as integer
						set itemY to (item 10 of rowFields) as integer
						set itemWidth to (item 11 of rowFields) as integer
						set itemHeight to (item 12 of rowFields) as integer
						set itemAngle to (item 14 of rowFields) as real

						tell currentSlide
							set nativeText to make new text item with properties {object text:itemText}
							set properties of every character of object text of nativeText to {font:itemFont, size:itemSize, color:{itemRed, itemGreen, itemBlue}}
							set properties of nativeText to {width:itemWidth, height:itemHeight, position:{itemX, itemY}}
							if rowType is "ROTATED_TEXT" then set rotation of nativeText to itemAngle
						end tell
					end if
				end if
			end repeat

			if (count of slides of deckDocument) is not expectedSlides then
				error "생성된 슬라이드 수가 PDF 페이지 수와 다릅니다."
			end if
			save deckDocument in POSIX file outputPath

			if keepOpen is false then
				close deckDocument saving no
				if keynoteWasRunning is false then quit
			end if
		end tell
	on error errorMessage number errorNumber
		tell application "Keynote"
			if deckDocument is not missing value then
				try
					close deckDocument saving no
				end try
			end if
			if keynoteWasRunning is false then
				try
					quit
				end try
			end if
		end tell
		error errorMessage number errorNumber
	end try

	return outputPath
end run
